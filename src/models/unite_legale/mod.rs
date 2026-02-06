pub mod common;
pub mod error;

use super::common::{Error as UpdatableError, UpdatableModel, copy_remote_zipped_csv};
use super::schema::unite_legale::dsl;
use crate::connectors::{Connectors, local::Connection};
use crate::update::utils::remote_file::RemoteFile;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use common::{
    SortDirection, UniteLegale, UniteLegaleSearchOutput, UniteLegaleSearchParams,
    UniteLegaleSearchResult, UniteLegaleSortField,
};
use diesel::pg::upsert::excluded;
use diesel::pg::{CopyFormat, CopyHeader};
use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types::{Date, Text};
use error::Error;

pub fn get(connection: &mut Connection, siren: &str) -> Result<UniteLegale, Error> {
    dsl::unite_legale
        .find(siren)
        .first::<UniteLegale>(connection)
        .map_err(|error| error.into())
}

pub fn search(
    connection: &mut Connection,
    params: &UniteLegaleSearchParams,
) -> Result<UniteLegaleSearchOutput, Error> {
    let has_q = params.q.is_some();

    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let offset = params.offset.unwrap_or(0).clamp(0, 100_000_000);

    // Build SELECT columns
    let mut select_columns = vec![
        "u.siren".to_string(),
        "u.etat_administratif".to_string(),
        "u.date_creation".to_string(),
        "u.denomination".to_string(),
        "u.denomination_usuelle_1".to_string(),
        "u.denomination_usuelle_2".to_string(),
        "u.denomination_usuelle_3".to_string(),
        "u.activite_principale".to_string(),
        "u.categorie_juridique".to_string(),
        "u.categorie_entreprise".to_string(),
    ];

    if has_q {
        select_columns.push("paradedb.score(u.ctid) AS score".to_string());
    } else {
        select_columns.push("NULL::real AS score".to_string());
    }

    // COUNT(*) OVER() is added by the outer CTE query, not here

    // Build WHERE conditions
    let mut conditions: Vec<String> = Vec::new();
    let mut param_index = 1u32;

    // Text search
    if has_q {
        conditions.push(format!("u.search_denomination ||| ${param_index}"));
        param_index += 1;
    }

    // Field filters
    let mut field_param_indices: Vec<(String, u32)> = Vec::new();

    if let Some(ref _v) = params.etat_administratif {
        conditions.push(format!("u.etat_administratif = ${param_index}"));
        field_param_indices.push(("etat_administratif".to_string(), param_index));
        param_index += 1;
    }
    if let Some(ref _v) = params.activite_principale {
        conditions.push(format!("u.activite_principale = ${param_index}"));
        field_param_indices.push(("activite_principale".to_string(), param_index));
        param_index += 1;
    }
    if let Some(ref _v) = params.categorie_juridique {
        conditions.push(format!("u.categorie_juridique = ${param_index}"));
        field_param_indices.push(("categorie_juridique".to_string(), param_index));
        param_index += 1;
    }
    if let Some(ref _v) = params.categorie_entreprise {
        conditions.push(format!("u.categorie_entreprise = ${param_index}"));
        field_param_indices.push(("categorie_entreprise".to_string(), param_index));
        param_index += 1;
    }
    if let Some(ref _v) = params.date_creation {
        conditions.push(format!("u.date_creation = ${param_index}"));
        field_param_indices.push(("date_creation".to_string(), param_index));
        param_index += 1;
    }
    if let Some(ref _v) = params.date_debut {
        conditions.push(format!("u.date_debut = ${param_index}"));
        field_param_indices.push(("date_debut".to_string(), param_index));
        param_index += 1;
    }

    let _ = param_index; // suppress unused warning

    // Build ORDER BY
    let sort_field = params.sort.unwrap_or(if has_q {
        UniteLegaleSortField::Relevance
    } else {
        UniteLegaleSortField::DateCreation
    });

    let resolved_dir = params.direction.unwrap_or(SortDirection::Desc);

    let order_by = match (sort_field, resolved_dir) {
        (UniteLegaleSortField::Relevance, SortDirection::Asc) => "score ASC",
        (UniteLegaleSortField::Relevance, SortDirection::Desc) => "score DESC",
        (UniteLegaleSortField::DateCreation, SortDirection::Asc) => {
            "u.date_creation ASC NULLS LAST"
        }
        (UniteLegaleSortField::DateCreation, SortDirection::Desc) => {
            "u.date_creation DESC NULLS LAST"
        }
        (UniteLegaleSortField::DateDebut, SortDirection::Asc) => "u.date_debut ASC NULLS LAST",
        (UniteLegaleSortField::DateDebut, SortDirection::Desc) => "u.date_debut DESC NULLS LAST",
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
        "WITH search_results AS (SELECT {} FROM unite_legale u {} ORDER BY {} LIMIT {} OFFSET {}) SELECT *, COUNT(*) OVER() AS total FROM search_results",
        select_columns.join(", "),
        where_clause,
        order_by,
        limit,
        offset
    );

    // Bind parameters in order
    let mut query = sql_query(&sql).into_boxed();

    if let Some(ref q) = params.q {
        query = query.bind::<Text, _>(q);
    }

    for (field_name, _) in &field_param_indices {
        match field_name.as_str() {
            "etat_administratif" => {
                let val = match params.etat_administratif.unwrap() {
                    common::EtatAdministratif::A => "A",
                    common::EtatAdministratif::F => "F",
                };
                query = query.bind::<Text, _>(val);
            }
            "activite_principale" => {
                query = query.bind::<Text, _>(params.activite_principale.as_ref().unwrap());
            }
            "categorie_juridique" => {
                query = query.bind::<Text, _>(params.categorie_juridique.as_ref().unwrap());
            }
            "categorie_entreprise" => {
                query = query.bind::<Text, _>(params.categorie_entreprise.as_ref().unwrap());
            }
            "date_creation" => {
                query = query.bind::<Date, _>(params.date_creation.unwrap());
            }
            "date_debut" => {
                query = query.bind::<Date, _>(params.date_debut.unwrap());
            }
            _ => {}
        }
    }

    let results = query
        .load::<UniteLegaleSearchResult>(connection)
        .map_err(|e| -> Error { e.into() })?;

    Ok(UniteLegaleSearchOutput {
        results,
        limit,
        offset,
        sort: sort_field,
        direction: resolved_dir,
    })
}

pub struct UniteLegaleModel {}

#[async_trait]
impl UpdatableModel for UniteLegaleModel {
    fn count(&self, connectors: &Connectors) -> Result<i64, UpdatableError> {
        let mut connection = connectors.local.pool.get()?;
        dsl::unite_legale
            .select(diesel::dsl::count(dsl::siren))
            .first::<i64>(&mut connection)
            .map_err(|error| error.into())
    }

    fn count_staging(&self, connectors: &Connectors) -> Result<i64, UpdatableError> {
        use super::schema::unite_legale_staging::dsl;

        let mut connection = connectors.local.pool.get()?;
        dsl::unite_legale_staging
            .select(diesel::dsl::count(dsl::siren))
            .first::<i64>(&mut connection)
            .map_err(|error| error.into())
    }

    fn insert_remote_file_in_staging(
        &self,
        connectors: &Connectors,
        remote_file: RemoteFile,
    ) -> Result<bool, UpdatableError> {
        use super::schema::unite_legale_staging::dsl;

        let mut connection = connectors.local.pool.get()?;

        sql_query("TRUNCATE unite_legale_staging").execute(&mut connection)?;

        diesel::copy_from(dsl::unite_legale_staging)
            .from_raw_data(
                (
                    dsl::siren,
                    dsl::statut_diffusion,
                    dsl::unite_purgee,
                    dsl::date_creation,
                    dsl::sigle,
                    dsl::sexe,
                    dsl::prenom_1,
                    dsl::prenom_2,
                    dsl::prenom_3,
                    dsl::prenom_4,
                    dsl::prenom_usuel,
                    dsl::pseudonyme,
                    dsl::identifiant_association,
                    dsl::tranche_effectifs,
                    dsl::annee_effectifs,
                    dsl::date_dernier_traitement,
                    dsl::nombre_periodes,
                    dsl::categorie_entreprise,
                    dsl::annee_categorie_entreprise,
                    dsl::date_debut,
                    dsl::etat_administratif,
                    dsl::nom,
                    dsl::nom_usage,
                    dsl::denomination,
                    dsl::denomination_usuelle_1,
                    dsl::denomination_usuelle_2,
                    dsl::denomination_usuelle_3,
                    dsl::categorie_juridique,
                    dsl::activite_principale,
                    dsl::nomenclature_activite_principale,
                    dsl::nic_siege,
                    dsl::economie_sociale_solidaire,
                    dsl::societe_mission,
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
            sql_query("ALTER TABLE unite_legale RENAME TO unite_legale_temp").execute(conn)?;
            sql_query("ALTER TABLE unite_legale_staging RENAME TO unite_legale").execute(conn)?;
            sql_query("ALTER TABLE unite_legale_temp RENAME TO unite_legale_staging")
                .execute(conn)?;
            sql_query("TRUNCATE unite_legale_staging").execute(conn)?;
            sql_query(
                r#"
                UPDATE group_metadata
                SET last_imported_timestamp = staging_imported_timestamp
                WHERE group_type = 'unites_legales'
                "#,
            )
            .execute(conn)?;
            sql_query(
                r#"
                UPDATE group_metadata
                SET staging_imported_timestamp = NULL
                WHERE group_type = 'unites_legales'
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

        Ok(insee.get_total_unites_legales(start_timestamp).await?)
    }

    // SELECT date_dernier_traitement FROM unite_legale WHERE date_dernier_traitement IS NOT NULL ORDER BY date_dernier_traitement DESC LIMIT 1;
    fn get_last_insee_synced_timestamp(
        &self,
        connectors: &Connectors,
    ) -> Result<Option<NaiveDateTime>, UpdatableError> {
        let mut connection = connectors.local.pool.get()?;
        dsl::unite_legale
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

        let (next_cursor, unites_legales) = insee
            .get_daily_unites_legales(start_timestamp, cursor)
            .await?;

        let mut connection = connectors.local.pool.get()?;

        let updated_count = diesel::insert_into(dsl::unite_legale)
            .values(&unites_legales)
            .on_conflict(dsl::siren)
            .do_update()
            .set((
                dsl::statut_diffusion.eq(excluded(dsl::statut_diffusion)),
                dsl::unite_purgee.eq(excluded(dsl::unite_purgee)),
                dsl::date_creation.eq(excluded(dsl::date_creation)),
                dsl::sigle.eq(excluded(dsl::sigle)),
                dsl::sexe.eq(excluded(dsl::sexe)),
                dsl::prenom_1.eq(excluded(dsl::prenom_1)),
                dsl::prenom_2.eq(excluded(dsl::prenom_2)),
                dsl::prenom_3.eq(excluded(dsl::prenom_3)),
                dsl::prenom_4.eq(excluded(dsl::prenom_4)),
                dsl::prenom_usuel.eq(excluded(dsl::prenom_usuel)),
                dsl::pseudonyme.eq(excluded(dsl::pseudonyme)),
                dsl::identifiant_association.eq(excluded(dsl::identifiant_association)),
                dsl::tranche_effectifs.eq(excluded(dsl::tranche_effectifs)),
                dsl::annee_effectifs.eq(excluded(dsl::annee_effectifs)),
                dsl::date_dernier_traitement.eq(excluded(dsl::date_dernier_traitement)),
                dsl::nombre_periodes.eq(excluded(dsl::nombre_periodes)),
                dsl::categorie_entreprise.eq(excluded(dsl::categorie_entreprise)),
                dsl::annee_categorie_entreprise.eq(excluded(dsl::annee_categorie_entreprise)),
                dsl::date_debut.eq(excluded(dsl::date_debut)),
                dsl::etat_administratif.eq(excluded(dsl::etat_administratif)),
                dsl::nom.eq(excluded(dsl::nom)),
                dsl::nom_usage.eq(excluded(dsl::nom_usage)),
                dsl::denomination.eq(excluded(dsl::denomination)),
                dsl::denomination_usuelle_1.eq(excluded(dsl::denomination_usuelle_1)),
                dsl::denomination_usuelle_2.eq(excluded(dsl::denomination_usuelle_2)),
                dsl::denomination_usuelle_3.eq(excluded(dsl::denomination_usuelle_3)),
                dsl::categorie_juridique.eq(excluded(dsl::categorie_juridique)),
                dsl::activite_principale.eq(excluded(dsl::activite_principale)),
                dsl::nomenclature_activite_principale
                    .eq(excluded(dsl::nomenclature_activite_principale)),
                dsl::nic_siege.eq(excluded(dsl::nic_siege)),
                dsl::economie_sociale_solidaire.eq(excluded(dsl::economie_sociale_solidaire)),
                dsl::societe_mission.eq(excluded(dsl::societe_mission)),
                dsl::caractere_employeur.eq(excluded(dsl::caractere_employeur)),
            ))
            .execute(&mut connection)?;

        Ok((next_cursor, updated_count))
    }
}
