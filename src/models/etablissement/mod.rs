pub mod common;
pub mod error;

use super::common::{Error as UpdatableError, UpdatableModel, copy_remote_zipped_csv};
use super::schema::etablissement::dsl;
use super::search;
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
use diesel_async::{AsyncConnection, RunQueryDsl};
use error::Error;
use tracing::info;

pub async fn get(connection: &mut Connection, siret: &str) -> Result<Etablissement, Error> {
    dsl::etablissement
        .find(siret)
        .select(Etablissement::as_select())
        .first::<Etablissement>(connection)
        .await
        .map_err(|error| error.into())
}

pub async fn get_with_siren(
    connection: &mut Connection,
    siren: &str,
) -> Result<Vec<Etablissement>, Error> {
    dsl::etablissement
        .filter(dsl::siren.eq(siren))
        .select(Etablissement::as_select())
        .load::<Etablissement>(connection)
        .await
        .map_err(|error| error.into())
}

pub async fn get_siege_with_siren(
    connection: &mut Connection,
    siren: &str,
) -> Result<Etablissement, Error> {
    dsl::etablissement
        .filter(dsl::siren.eq(siren).and(dsl::etablissement_siege.eq(true)))
        .select(Etablissement::as_select())
        .first::<Etablissement>(connection)
        .await
        .map_err(|error| error.into())
}

/// Resolved parameters of one search execution.
struct SearchPlan<'a> {
    text: search::TextMatch<'a>,
    commune_codes: Option<&'a [String]>,
    facets: &'a [String],
    sort: EtablissementSortField,
    direction: SortDirection,
    limit: i64,
    offset: i64,
}

pub async fn search(
    connection: &mut Connection,
    params: &EtablissementSearchParams,
) -> Result<EtablissementSearchOutput, Error> {
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let offset = params.offset.unwrap_or(0).clamp(0, 10_000);

    let q = params
        .q
        .as_deref()
        .map(str::trim)
        .filter(|q| q.chars().count() >= search::MIN_QUERY_LENGTH);

    let sort = params.sort.unwrap_or(if q.is_some() {
        EtablissementSortField::Relevance
    } else {
        EtablissementSortField::DateCreation
    });

    let direction = params.direction.unwrap_or(match sort {
        EtablissementSortField::Distance => SortDirection::Asc,
        _ => SortDirection::Desc,
    });

    let facets = search::requested_facets(
        params.facette.as_deref(),
        search::ETABLISSEMENT_FACET_FIELDS,
    );

    // A commune label matching nothing can yield nothing: no point querying the
    // large table.
    let commune_codes = match params
        .commune
        .as_deref()
        .map(str::trim)
        .filter(|commune| !commune.is_empty())
    {
        Some(commune) => {
            let codes = search::resolve_commune(connection, commune).await?;
            if codes.is_empty() {
                return Ok(EtablissementSearchOutput {
                    results: Vec::new(),
                    total: 0,
                    total_capped: false,
                    limit,
                    offset,
                    sort,
                    direction,
                    suggestion: None,
                    facettes: search::Facets::new(),
                });
            }
            Some(codes)
        }
        None => None,
    };

    let parsed = match q {
        Some(q) => Some(search::parse_query(connection, q, search::SOURCE_ETABLISSEMENT).await?),
        None => None,
    };

    let mut plan = SearchPlan {
        text: match parsed.as_ref().and_then(|parsed| parsed.tsquery.as_deref()) {
            Some(tsquery) => search::TextMatch::FullText(tsquery),
            None => search::TextMatch::None,
        },
        commune_codes: commune_codes.as_deref(),
        facets: &facets,
        sort,
        direction,
        limit,
        offset,
    };

    let mut output = execute(connection, params, &plan).await?;

    // Covers what lexicon correction cannot: transpositions and infix matches.
    // Restricted to the first page — an empty page further in means the caller
    // paged past the end, not that the search found nothing, and retrying there
    // would scan the whole table for results that already exist.
    if output.results.is_empty()
        && offset == 0
        && let Some(q) = q
    {
        info!(
            target: "sirene::search",
            source = search::SOURCE_ETABLISSEMENT,
            query_length = q.chars().count(),
            "repli trigramme declenche"
        );

        plan.text = search::TextMatch::Trigram(q);
        output = execute(connection, params, &plan).await?;
    }

    // Offered systematically, the suggestion would be wrong for ~45% of rare but
    // correct names, so it only surfaces when the search found nothing.
    if output.results.is_empty() {
        output.suggestion = parsed.and_then(|parsed| parsed.suggestion);
    }

    Ok(output)
}

async fn execute(
    connection: &mut Connection,
    params: &EtablissementSearchParams,
    plan: &SearchPlan<'_>,
) -> Result<EtablissementSearchOutput, Error> {
    let mut binder = search::Binder::new();
    let mut conditions: Vec<String> = Vec::new();

    // Declared first so the expression is constant at planning time, which the
    // GiST KNN walk requires.
    let reference_point = match (params.lng, params.lat, params.radius) {
        (Some(lng), Some(lat), Some(radius)) => {
            let lng = binder.push(search::Bind::Float8(lng));
            let lat = binder.push(search::Bind::Float8(lat));
            let point = format!("ST_SetSRID(ST_MakePoint({lng}, {lat}), 4326)::geography");
            let radius = binder.push(search::Bind::Float8(radius));
            conditions.push(format!("ST_DWithin(e.position, {point}, {radius})"));
            Some(point)
        }
        _ => None,
    };

    let columns = search::ETABLISSEMENT_SEARCH_COLUMNS;
    let score = match plan.text {
        search::TextMatch::None => "NULL::real".to_string(),
        search::TextMatch::FullText(tsquery) => {
            let vector = search::search_vector(Some("e"), columns);
            let placeholder = binder.push(search::Bind::TsQuery(tsquery.to_string()));
            conditions.push(format!("{vector} @@ {placeholder}::tsquery"));
            format!("ts_rank_cd({vector}, {placeholder}::tsquery)")
        }
        search::TextMatch::Trigram(q) => {
            let trigram = search::search_trigram(Some("e"), columns);
            let placeholder = binder.text(q);
            let needle = format!("lower(public.immutable_unaccent({placeholder}))");
            conditions.push(format!("{needle} <% {trigram}"));
            format!("word_similarity({needle}, {trigram})")
        }
    };

    let distance = match &reference_point {
        Some(point) => format!("ST_Distance(e.position, {point})"),
        None => "NULL::float8".to_string(),
    };

    // Kept out of `conditions` until the query shape is chosen: the lateral form
    // replaces it with a join, and recovering that by matching generated SQL
    // would silently break the day the predicate is spelled differently.
    let commune = plan.commune_codes.map(|codes| {
        let placeholder = binder.push(search::Bind::TextArray(codes.to_vec()));
        (
            placeholder.clone(),
            format!("e.code_commune = ANY({placeholder})"),
        )
    });

    if let Some(value) = params.etat_administratif {
        let placeholder = binder.text(match value {
            common::EtatAdministratif::A => "A",
            common::EtatAdministratif::F => "F",
        });
        conditions.push(format!("e.etat_administratif = {placeholder}"));
    }
    if let Some(value) = params.etablissement_siege {
        let placeholder = binder.push(search::Bind::Bool(value));
        conditions.push(format!("e.etablissement_siege = {placeholder}"));
    }

    binder.filter_in(
        &mut conditions,
        "e.code_postal",
        params.code_postal.as_deref(),
    );
    binder.filter_in(&mut conditions, "e.siren", params.siren.as_deref());
    binder.filter_in(
        &mut conditions,
        "e.code_commune",
        params.code_commune.as_deref(),
    );
    binder.filter_in(
        &mut conditions,
        "e.activite_principale",
        params.activite_principale.as_deref(),
    );

    binder.filter_not_in(
        &mut conditions,
        "e.code_postal",
        params.code_postal_not.as_deref(),
    );
    binder.filter_not_in(
        &mut conditions,
        "e.code_commune",
        params.code_commune_not.as_deref(),
    );
    binder.filter_not_in(
        &mut conditions,
        "e.activite_principale",
        params.activite_principale_not.as_deref(),
    );

    binder.range(
        &mut conditions,
        "e.date_creation",
        params.date_creation_min,
        params.date_creation_max,
    );
    binder.range(
        &mut conditions,
        "e.date_debut",
        params.date_debut_min,
        params.date_debut_max,
    );

    let order_by = order_by_clause(plan.sort, plan.direction, reference_point.as_deref());

    // Filtering by commune without a text filter means sorting millions of rows:
    // `code_commune = ANY(...)` cannot yield index order. A top-N per commune,
    // merged afterwards, takes 1.5 ms where the plain form took 22 s.
    let lateral = matches!(
        (&commune, &plan.text, &reference_point),
        (Some(_), search::TextMatch::None, None)
    );

    let projection = format!(
        "e.siret, e.siren, e.etat_administratif, e.date_creation, e.denomination_usuelle, \
         e.enseigne_1, e.enseigne_2, e.enseigne_3, e.code_postal, e.libelle_commune, \
         e.activite_principale, e.etablissement_siege, e.position, \
         {distance} AS meter_distance, {score} AS score"
    );

    let sql = match (&commune, lateral) {
        (Some((placeholder, _)), true) => {
            let mut inner = vec!["e.code_commune = codes.code_commune".to_string()];
            inner.extend(conditions.iter().cloned());

            format!(
                "SELECT {projection} FROM unnest({placeholder}) AS codes(code_commune) \
                 CROSS JOIN LATERAL ( \
                   SELECT * FROM etablissement e WHERE {} ORDER BY {order_by} LIMIT {} \
                 ) e \
                 ORDER BY {order_by} LIMIT {} OFFSET {}",
                inner.join(" AND "),
                plan.limit + plan.offset,
                plan.limit,
                plan.offset
            )
        }
        _ => format!(
            "SELECT {projection} FROM etablissement e {} \
             ORDER BY {order_by} LIMIT {} OFFSET {}",
            where_clause(&conditions, commune.as_ref()),
            plan.limit,
            plan.offset
        ),
    };

    let results = binder
        .apply(sql_query(sql).into_boxed())
        .load::<EtablissementSearchResult>(connection)
        .await
        .map_err(|error| -> Error { error.into() })?;

    // The count and the facets always take the plain form, lateral or not.
    let filters = where_clause(&conditions, commune.as_ref());
    let (total, total_capped) =
        search::capped_total(connection, &binder, "etablissement", "e", &filters).await;
    let facettes = search::compute_facets(
        connection,
        &binder,
        "etablissement",
        "e",
        &filters,
        plan.facets,
    )
    .await;

    Ok(EtablissementSearchOutput {
        results,
        total,
        total_capped,
        limit: plan.limit,
        offset: plan.offset,
        sort: plan.sort,
        direction: plan.direction,
        suggestion: None,
        facettes,
    })
}

fn where_clause(conditions: &[String], commune: Option<&(String, String)>) -> String {
    let mut all: Vec<&str> = conditions.iter().map(String::as_str).collect();
    if let Some((_, condition)) = commune {
        all.push(condition);
    }
    if all.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", all.join(" AND "))
    }
}

fn order_by_clause(
    sort: EtablissementSortField,
    direction: SortDirection,
    reference_point: Option<&str>,
) -> String {
    let dir = match direction {
        SortDirection::Asc => "ASC",
        SortDirection::Desc => "DESC",
    };

    match sort {
        EtablissementSortField::Distance => match reference_point {
            Some(point) => format!("e.position <-> {point} {dir}"),
            None => format!("e.date_creation {dir} NULLS LAST"),
        },
        EtablissementSortField::Relevance => format!("score {dir}"),
        EtablissementSortField::DateCreation => format!("e.date_creation {dir} NULLS LAST"),
        EtablissementSortField::DateDebut => format!("e.date_debut {dir} NULLS LAST"),
    }
}

pub struct EtablissementModel {}

#[async_trait]
impl UpdatableModel for EtablissementModel {
    async fn count(&self, connectors: &Connectors) -> Result<i64, UpdatableError> {
        let mut connection = connectors.local.pool.get().await?;
        dsl::etablissement
            .select(diesel::dsl::count(dsl::siret))
            .first::<i64>(&mut connection)
            .await
            .map_err(|error| error.into())
    }

    async fn count_staging(&self, connectors: &Connectors) -> Result<i64, UpdatableError> {
        use super::schema::etablissement_staging::dsl;

        let mut connection = connectors.local.pool.get().await?;
        dsl::etablissement_staging
            .select(diesel::dsl::count(dsl::siret))
            .first::<i64>(&mut connection)
            .await
            .map_err(|error| error.into())
    }

    fn insert_remote_file_in_staging(
        &self,
        connectors: &Connectors,
        remote_file: RemoteFile,
    ) -> Result<bool, UpdatableError> {
        use super::schema::etablissement_staging::dsl;
        use diesel::Connection as _;
        use diesel::ExecuteCopyFromDsl as SyncExecuteCopy;
        use diesel::RunQueryDsl as SyncRunQueryDsl;

        tokio::task::block_in_place(|| {
            let mut connection =
                diesel::pg::PgConnection::establish(&connectors.local.database_url)
                    .map_err(|_| UpdatableError::SyncConnectionFailed)?;

            super::common::load_staging_without_indexes(
                &mut connection,
                "etablissement_staging",
                |connection| {
                    SyncRunQueryDsl::execute(
                        diesel::sql_query("TRUNCATE etablissement_staging"),
                        connection,
                    )
                    .map_err(|e| UpdatableError::Database { source: e })?;

                    let copy_query = diesel::copy_from(dsl::etablissement_staging)
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
                        .with_header(CopyHeader::Set(true));
                    SyncExecuteCopy::execute(copy_query, connection)
                        .map(|count| count > 0)
                        .map_err(|e| UpdatableError::Database { source: e })
                },
            )
        })
    }

    async fn swap(&self, connectors: &Connectors) -> Result<(), UpdatableError> {
        let mut connection = connectors.local.pool.get().await?;
        connection
            .transaction(async |conn| {
                sql_query("ALTER TABLE etablissement RENAME TO etablissement_temp")
                    .execute(conn)
                    .await?;
                sql_query("ALTER TABLE etablissement_staging RENAME TO etablissement")
                    .execute(conn)
                    .await?;
                sql_query("ALTER TABLE etablissement_temp RENAME TO etablissement_staging")
                    .execute(conn)
                    .await?;
                sql_query("TRUNCATE etablissement_staging")
                    .execute(conn)
                    .await?;
                sql_query(
                    r#"
                UPDATE group_metadata
                SET last_imported_timestamp = staging_imported_timestamp
                WHERE group_type = 'etablissements'
                "#,
                )
                .execute(conn)
                .await?;
                sql_query(
                    r#"
                UPDATE group_metadata
                SET staging_imported_timestamp = NULL
                WHERE group_type = 'etablissements'
                "#,
                )
                .execute(conn)
                .await?;

                diesel::QueryResult::Ok(())
            })
            .await
            .map_err(|e| UpdatableError::Database { source: e })
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

    async fn get_last_insee_synced_timestamp(
        &self,
        connectors: &Connectors,
    ) -> Result<Option<NaiveDateTime>, UpdatableError> {
        let mut connection = connectors.local.pool.get().await?;
        dsl::etablissement
            .select(dsl::date_dernier_traitement)
            .order(dsl::date_dernier_traitement.desc())
            .filter(dsl::date_dernier_traitement.is_not_null())
            .first::<Option<NaiveDateTime>>(&mut connection)
            .await
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

        let mut connection = connectors.local.pool.get().await?;

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
            .execute(&mut connection)
            .await?;

        Ok((next_cursor, updated_count))
    }

    fn search_source(&self) -> Option<&'static str> {
        Some(search::SOURCE_ETABLISSEMENT)
    }
}
