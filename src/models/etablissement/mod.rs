pub mod common;
pub mod error;

use super::common::{Error as UpdatableError, UpdatableModel, copy_remote_zipped_csv};
use super::schema::etablissement::dsl;
use crate::connectors::{Connectors, local::Connection};
use crate::update::utils::remote_file::RemoteFile;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use common::{
    Etablissement, EtablissementSearchOutput, EtablissementSearchParams, EtablissementSearchResult,
    EtablissementSortField, SortDirection,
};
use diesel::pg::upsert::excluded;
use diesel::pg::{CopyFormat, CopyHeader};
use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types::{Bool, Float8, Text};
use error::Error;

pub fn get(connection: &mut Connection, siret: &str) -> Result<Etablissement, Error> {
    dsl::etablissement
        .find(siret)
        .select(Etablissement::as_select())
        .first::<Etablissement>(connection)
        .map_err(|error| error.into())
}

pub fn get_with_siren(
    connection: &mut Connection,
    siren: &str,
) -> Result<Vec<Etablissement>, Error> {
    dsl::etablissement
        .filter(dsl::siren.eq(siren))
        .select(Etablissement::as_select())
        .load::<Etablissement>(connection)
        .map_err(|error| error.into())
}

pub fn get_siege_with_siren(
    connection: &mut Connection,
    siren: &str,
) -> Result<Etablissement, Error> {
    dsl::etablissement
        .filter(dsl::siren.eq(siren).and(dsl::etablissement_siege.eq(true)))
        .select(Etablissement::as_select())
        .first::<Etablissement>(connection)
        .map_err(|error| error.into())
}

pub fn search(
    connection: &mut Connection,
    params: &EtablissementSearchParams,
) -> Result<EtablissementSearchOutput, Error> {
    let has_geo = params.lat.is_some() && params.lng.is_some() && params.radius.is_some();
    let has_q = params.q.is_some();

    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let offset = params.offset.unwrap_or(0).clamp(0, 10_000);

    // Build SELECT columns
    let mut select_columns = vec![
        "e.siret".to_string(),
        "e.siren".to_string(),
        "e.etat_administratif".to_string(),
        "e.date_creation".to_string(),
        "e.denomination_usuelle".to_string(),
        "e.enseigne_1".to_string(),
        "e.enseigne_2".to_string(),
        "e.enseigne_3".to_string(),
        "e.code_postal".to_string(),
        "e.libelle_commune".to_string(),
        "e.activite_principale".to_string(),
        "e.etablissement_siege".to_string(),
        "e.position".to_string(),
    ];

    if has_geo {
        select_columns.push("ST_Distance(e.position, ref_point.pt) AS meter_distance".to_string());
    } else {
        select_columns.push("NULL::float8 AS meter_distance".to_string());
    }

    if has_q {
        select_columns.push("paradedb.score(e.ctid) AS score".to_string());
    } else {
        select_columns.push("NULL::real AS score".to_string());
    }

    // COUNT(*) OVER() is added by the outer CTE query, not here

    // Build FROM clause
    let mut from_parts = vec!["etablissement e".to_string()];
    let mut param_index = 1u32;

    if has_geo {
        from_parts.push(format!(
            "(SELECT ST_SetSRID(ST_MakePoint(${}, ${}), 4326)::geography AS pt) ref_point",
            param_index,
            param_index + 1
        ));
        param_index += 2;
        // radius param will be used in WHERE
    }

    // Build WHERE conditions
    let mut conditions: Vec<String> = Vec::new();

    // Text search
    let q_param_index;
    if has_q {
        q_param_index = Some(param_index);
        conditions.push(format!(
            "(e.search_denomination ||| ${param_index} OR e.libelle_commune ||| ${param_index})"
        ));
        param_index += 1;
    } else {
        q_param_index = None;
    }

    // Geo filter
    let radius_param_index;
    if has_geo {
        radius_param_index = Some(param_index);
        conditions.push(format!(
            "ST_DWithin(e.position, ref_point.pt, ${param_index})"
        ));
        param_index += 1;
    } else {
        radius_param_index = None;
    }

    // Field filters
    let mut field_param_indices: Vec<(String, u32)> = Vec::new();

    if let Some(ref _v) = params.etat_administratif {
        conditions.push(format!("e.etat_administratif = ${param_index}"));
        field_param_indices.push(("etat_administratif".to_string(), param_index));
        param_index += 1;
    }
    if let Some(ref _v) = params.code_postal {
        conditions.push(format!("e.code_postal = ${param_index}"));
        field_param_indices.push(("code_postal".to_string(), param_index));
        param_index += 1;
    }
    if let Some(ref _v) = params.siren {
        conditions.push(format!("e.siren = ${param_index}"));
        field_param_indices.push(("siren".to_string(), param_index));
        param_index += 1;
    }
    if let Some(ref _v) = params.code_commune {
        conditions.push(format!("e.code_commune = ${param_index}"));
        field_param_indices.push(("code_commune".to_string(), param_index));
        param_index += 1;
    }
    if let Some(ref _v) = params.activite_principale {
        conditions.push(format!("e.activite_principale = ${param_index}"));
        field_param_indices.push(("activite_principale".to_string(), param_index));
        param_index += 1;
    }
    if let Some(ref _v) = params.etablissement_siege {
        conditions.push(format!("e.etablissement_siege = ${param_index}"));
        field_param_indices.push(("etablissement_siege".to_string(), param_index));
        param_index += 1;
    }

    let _ = param_index; // suppress unused warning

    // Build ORDER BY
    let sort_field = params.sort.unwrap_or(if has_q {
        EtablissementSortField::Relevance
    } else {
        EtablissementSortField::DateCreation
    });

    let resolved_dir = params.direction.unwrap_or(match sort_field {
        EtablissementSortField::Distance => SortDirection::Asc,
        _ => SortDirection::Desc,
    });

    let order_by = match (sort_field, resolved_dir) {
        (EtablissementSortField::Distance, SortDirection::Desc) => {
            "e.position <-> ref_point.pt DESC"
        }
        (EtablissementSortField::Distance, SortDirection::Asc) => "e.position <-> ref_point.pt ASC",
        (EtablissementSortField::Relevance, SortDirection::Asc) => "score ASC",
        (EtablissementSortField::Relevance, SortDirection::Desc) => "score DESC",
        (EtablissementSortField::DateCreation, SortDirection::Asc) => {
            "e.date_creation ASC NULLS LAST"
        }
        (EtablissementSortField::DateCreation, SortDirection::Desc) => {
            "e.date_creation DESC NULLS LAST"
        }
        (EtablissementSortField::DateDebut, SortDirection::Asc) => "e.date_debut ASC NULLS LAST",
        (EtablissementSortField::DateDebut, SortDirection::Desc) => "e.date_debut DESC NULLS LAST",
    };

    // Assemble query — use a CTE to isolate the search from the COUNT window function,
    // because ParadeDB's custom scan planner rejects unsupported query shapes when
    // COUNT(*) OVER() is combined with BM25 index scans.
    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let sql = format!(
        "WITH search_results AS (SELECT {} FROM {} {}) SELECT *, COUNT(*) OVER() AS total FROM search_results ORDER BY {} LIMIT {} OFFSET {}",
        select_columns.join(", "),
        from_parts.join(", "),
        where_clause,
        order_by,
        limit,
        offset
    );

    // Bind parameters in order
    let mut query = sql_query(&sql).into_boxed();

    if has_geo {
        query = query
            .bind::<Float8, _>(params.lng.unwrap())
            .bind::<Float8, _>(params.lat.unwrap());
    }

    if let Some(ref q) = params.q {
        let _ = q_param_index;
        query = query.bind::<Text, _>(q);
    }

    if has_geo {
        let _ = radius_param_index;
        query = query.bind::<Float8, _>(params.radius.unwrap());
    }

    // Bind field filters in order
    for (field_name, _) in &field_param_indices {
        match field_name.as_str() {
            "etat_administratif" => {
                let val = match params.etat_administratif.unwrap() {
                    common::EtatAdministratif::A => "A",
                    common::EtatAdministratif::F => "F",
                };
                query = query.bind::<Text, _>(val);
            }
            "code_postal" => {
                query = query.bind::<Text, _>(params.code_postal.as_ref().unwrap());
            }
            "siren" => {
                query = query.bind::<Text, _>(params.siren.as_ref().unwrap());
            }
            "code_commune" => {
                query = query.bind::<Text, _>(params.code_commune.as_ref().unwrap());
            }
            "activite_principale" => {
                query = query.bind::<Text, _>(params.activite_principale.as_ref().unwrap());
            }
            "etablissement_siege" => {
                query = query.bind::<Bool, _>(params.etablissement_siege.unwrap());
            }
            _ => {}
        }
    }

    let results = query
        .load::<EtablissementSearchResult>(connection)
        .map_err(|e| -> Error { e.into() })?;

    Ok(EtablissementSearchOutput {
        results,
        limit,
        offset,
        sort: sort_field,
        direction: resolved_dir,
    })
}

pub struct EtablissementModel {}

#[async_trait]
impl UpdatableModel for EtablissementModel {
    fn count(&self, connectors: &Connectors) -> Result<i64, UpdatableError> {
        let mut connection = connectors.local.pool.get()?;
        dsl::etablissement
            .select(diesel::dsl::count(dsl::siret))
            .first::<i64>(&mut connection)
            .map_err(|error| error.into())
    }

    fn count_staging(&self, connectors: &Connectors) -> Result<i64, UpdatableError> {
        use super::schema::etablissement_staging::dsl;

        let mut connection = connectors.local.pool.get()?;
        dsl::etablissement_staging
            .select(diesel::dsl::count(dsl::siret))
            .first::<i64>(&mut connection)
            .map_err(|error| error.into())
    }

    fn insert_remote_file_in_staging(
        &self,
        connectors: &Connectors,
        remote_file: RemoteFile,
    ) -> Result<bool, UpdatableError> {
        use super::schema::etablissement_staging::dsl;

        let mut connection = connectors.local.pool.get()?;

        sql_query("TRUNCATE etablissement_staging").execute(&mut connection)?;

        diesel::copy_from(dsl::etablissement_staging)
            .from_raw_data(
                (
                    dsl::siren,
                    dsl::nic,
                    dsl::siret,
                    dsl::statut_diffusion,
                    dsl::date_creation,
                    dsl::tranche_effectifs,
                    dsl::annee_effectifs,
                    dsl::activite_principale_registre_metiers,
                    dsl::date_dernier_traitement,
                    dsl::etablissement_siege,
                    dsl::nombre_periodes,
                    dsl::complement_adresse,
                    dsl::numero_voie,
                    dsl::indice_repetition,
                    dsl::dernier_numero_voie,
                    dsl::indice_repetition_dernier_numero_voie,
                    dsl::type_voie,
                    dsl::libelle_voie,
                    dsl::code_postal,
                    dsl::libelle_commune,
                    dsl::libelle_commune_etranger,
                    dsl::distribution_speciale,
                    dsl::code_commune,
                    dsl::code_cedex,
                    dsl::libelle_cedex,
                    dsl::code_pays_etranger,
                    dsl::libelle_pays_etranger,
                    dsl::identifiant_adresse,
                    dsl::coordonnee_lambert_x,
                    dsl::coordonnee_lambert_y,
                    dsl::complement_adresse2,
                    dsl::numero_voie_2,
                    dsl::indice_repetition_2,
                    dsl::type_voie_2,
                    dsl::libelle_voie_2,
                    dsl::code_postal_2,
                    dsl::libelle_commune_2,
                    dsl::libelle_commune_etranger_2,
                    dsl::distribution_speciale_2,
                    dsl::code_commune_2,
                    dsl::code_cedex_2,
                    dsl::libelle_cedex_2,
                    dsl::code_pays_etranger_2,
                    dsl::libelle_pays_etranger_2,
                    dsl::date_debut,
                    dsl::etat_administratif,
                    dsl::enseigne_1,
                    dsl::enseigne_2,
                    dsl::enseigne_3,
                    dsl::denomination_usuelle,
                    dsl::activite_principale,
                    dsl::nomenclature_activite_principale,
                    dsl::caractere_employeur,
                    dsl::activite_principale_naf25,
                ),
                |write| copy_remote_zipped_csv(remote_file.to_reader(), write),
            )
            .with_delimiter(',')
            .with_format(CopyFormat::Csv)
            .with_header(CopyHeader::Set(true))
            .execute(&mut connection)
            .map(|count| count > 0)
            .map_err(|error| error.into())
    }

    fn swap(&self, connectors: &Connectors) -> Result<(), UpdatableError> {
        let mut connection = connectors.local.pool.get()?;
        connection.build_transaction().read_write().run(|conn| {
            sql_query("ALTER TABLE etablissement RENAME TO etablissement_temp").execute(conn)?;
            sql_query("ALTER TABLE etablissement_staging RENAME TO etablissement").execute(conn)?;
            sql_query("ALTER TABLE etablissement_temp RENAME TO etablissement_staging")
                .execute(conn)?;
            sql_query("TRUNCATE etablissement_staging").execute(conn)?;
            sql_query(
                r#"
            UPDATE group_metadata
            SET last_imported_timestamp = staging_imported_timestamp
            WHERE group_type = 'etablissements'
            "#,
            )
            .execute(conn)?;
            sql_query(
                r#"
            UPDATE group_metadata
            SET staging_imported_timestamp = NULL
            WHERE group_type = 'etablissements'
            "#,
            )
            .execute(conn)?;

            Ok(())
        })
    }

    async fn get_total_count(
        &self,
        connectors: &mut Connectors,
        start_timestamp: NaiveDateTime,
    ) -> Result<u32, UpdatableError> {
        let insee = connectors
            .insee
            .as_mut()
            .ok_or(UpdatableError::MissingInseeConnector)?;

        Ok(insee.get_total_etablissements(start_timestamp).await?)
    }

    // SELECT date_dernier_traitement FROM etablissement WHERE date_dernier_traitement IS NOT NULL ORDER BY date_dernier_traitement DESC LIMIT 1;
    fn get_last_insee_synced_timestamp(
        &self,
        connectors: &Connectors,
    ) -> Result<Option<NaiveDateTime>, UpdatableError> {
        let mut connection = connectors.local.pool.get()?;
        dsl::etablissement
            .select(dsl::date_dernier_traitement)
            .order(dsl::date_dernier_traitement.desc())
            .filter(dsl::date_dernier_traitement.is_not_null())
            .first::<Option<NaiveDateTime>>(&mut connection)
            .map_err(|error| error.into())
    }

    async fn update_daily_data(
        &self,
        connectors: &mut Connectors,
        start_timestamp: NaiveDateTime,
        cursor: String,
    ) -> Result<(Option<String>, usize), UpdatableError> {
        let insee = connectors
            .insee
            .as_mut()
            .ok_or(UpdatableError::MissingInseeConnector)?;

        let (next_cursor, etablissements) = insee
            .get_daily_etablissements(start_timestamp, cursor)
            .await?;

        let mut connection = connectors.local.pool.get()?;

        let updated_count = diesel::insert_into(dsl::etablissement)
            .values(&etablissements)
            .on_conflict(dsl::siret)
            .do_update()
            .set((
                dsl::nic.eq(excluded(dsl::nic)),
                dsl::siren.eq(excluded(dsl::siren)),
                dsl::statut_diffusion.eq(excluded(dsl::statut_diffusion)),
                dsl::date_creation.eq(excluded(dsl::date_creation)),
                dsl::tranche_effectifs.eq(excluded(dsl::tranche_effectifs)),
                dsl::annee_effectifs.eq(excluded(dsl::annee_effectifs)),
                dsl::activite_principale_registre_metiers
                    .eq(excluded(dsl::activite_principale_registre_metiers)),
                dsl::date_dernier_traitement.eq(excluded(dsl::date_dernier_traitement)),
                dsl::etablissement_siege.eq(excluded(dsl::etablissement_siege)),
                dsl::nombre_periodes.eq(excluded(dsl::nombre_periodes)),
                dsl::complement_adresse.eq(excluded(dsl::complement_adresse)),
                dsl::numero_voie.eq(excluded(dsl::numero_voie)),
                dsl::indice_repetition.eq(excluded(dsl::indice_repetition)),
                dsl::type_voie.eq(excluded(dsl::type_voie)),
                dsl::libelle_voie.eq(excluded(dsl::libelle_voie)),
                dsl::code_postal.eq(excluded(dsl::code_postal)),
                dsl::libelle_commune.eq(excluded(dsl::libelle_commune)),
                dsl::libelle_commune_etranger.eq(excluded(dsl::libelle_commune_etranger)),
                dsl::distribution_speciale.eq(excluded(dsl::distribution_speciale)),
                dsl::code_commune.eq(excluded(dsl::code_commune)),
                dsl::code_cedex.eq(excluded(dsl::code_cedex)),
                dsl::libelle_cedex.eq(excluded(dsl::libelle_cedex)),
                dsl::code_pays_etranger.eq(excluded(dsl::code_pays_etranger)),
                dsl::libelle_pays_etranger.eq(excluded(dsl::libelle_pays_etranger)),
                dsl::complement_adresse2.eq(excluded(dsl::complement_adresse2)),
                dsl::numero_voie_2.eq(excluded(dsl::numero_voie_2)),
                dsl::indice_repetition_2.eq(excluded(dsl::indice_repetition_2)),
                dsl::type_voie_2.eq(excluded(dsl::type_voie_2)),
                dsl::libelle_voie_2.eq(excluded(dsl::libelle_voie_2)),
                dsl::code_postal_2.eq(excluded(dsl::code_postal_2)),
                dsl::libelle_commune_2.eq(excluded(dsl::libelle_commune_2)),
                dsl::libelle_commune_etranger_2.eq(excluded(dsl::libelle_commune_etranger_2)),
                dsl::distribution_speciale_2.eq(excluded(dsl::distribution_speciale_2)),
                dsl::code_commune_2.eq(excluded(dsl::code_commune_2)),
                dsl::code_cedex_2.eq(excluded(dsl::code_cedex_2)),
                dsl::libelle_cedex_2.eq(excluded(dsl::libelle_cedex_2)),
                dsl::code_pays_etranger_2.eq(excluded(dsl::code_pays_etranger_2)),
                dsl::libelle_pays_etranger_2.eq(excluded(dsl::libelle_pays_etranger_2)),
                dsl::date_debut.eq(excluded(dsl::date_debut)),
                dsl::etat_administratif.eq(excluded(dsl::etat_administratif)),
                dsl::enseigne_1.eq(excluded(dsl::enseigne_1)),
                dsl::enseigne_2.eq(excluded(dsl::enseigne_2)),
                dsl::enseigne_3.eq(excluded(dsl::enseigne_3)),
                dsl::denomination_usuelle.eq(excluded(dsl::denomination_usuelle)),
                dsl::activite_principale.eq(excluded(dsl::activite_principale)),
                dsl::nomenclature_activite_principale
                    .eq(excluded(dsl::nomenclature_activite_principale)),
                dsl::caractere_employeur.eq(excluded(dsl::caractere_employeur)),
                dsl::dernier_numero_voie.eq(excluded(dsl::dernier_numero_voie)),
                dsl::indice_repetition_dernier_numero_voie
                    .eq(excluded(dsl::indice_repetition_dernier_numero_voie)),
                dsl::identifiant_adresse.eq(excluded(dsl::identifiant_adresse)),
                dsl::coordonnee_lambert_x.eq(excluded(dsl::coordonnee_lambert_x)),
                dsl::coordonnee_lambert_y.eq(excluded(dsl::coordonnee_lambert_y)),
            ))
            .execute(&mut connection)?;

        Ok((next_cursor, updated_count))
    }
}
