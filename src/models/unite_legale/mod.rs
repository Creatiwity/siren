pub mod common;
pub mod error;

use super::common::{Error as UpdatableError, UpdatableModel, copy_remote_zipped_csv};
use super::schema::unite_legale::dsl;
use super::search;
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
use diesel_async::{AsyncConnection, RunQueryDsl};
use error::Error;
use tracing::info;

pub async fn get(connection: &mut Connection, siren: &str) -> Result<UniteLegale, Error> {
    dsl::unite_legale
        .find(siren)
        .select(UniteLegale::as_select())
        .first::<UniteLegale>(connection)
        .await
        .map_err(|error| error.into())
}

/// Resolved parameters of one search execution.
struct SearchPlan<'a> {
    text: search::TextMatch<'a>,
    facets: &'a [String],
    cursor: Option<&'a str>,
    sort: UniteLegaleSortField,
    direction: SortDirection,
    limit: i64,
    offset: i64,
}

pub async fn search(
    connection: &mut Connection,
    params: &UniteLegaleSearchParams,
) -> Result<UniteLegaleSearchOutput, Error> {
    let cursor = params
        .cursor
        .as_deref()
        .map(str::trim)
        .filter(|cursor| !cursor.is_empty())
        .and_then(search::decode_cursor);

    // Primary-key order costs a bounded index walk per page, so it can afford
    // wider pages than offset pagination.
    let max_limit = match params.sort {
        Some(UniteLegaleSortField::Siren) => search::CURSOR_LIMIT_MAX,
        _ => 100,
    };
    let limit = params.limit.unwrap_or(20).clamp(1, max_limit);
    let offset = if cursor.is_some() {
        0
    } else {
        params.offset.unwrap_or(0).clamp(0, 10_000)
    };

    let q = params
        .q
        .as_deref()
        .map(str::trim)
        .filter(|q| q.chars().count() >= search::MIN_QUERY_LENGTH);

    let sort = params.sort.unwrap_or(if q.is_some() {
        UniteLegaleSortField::Relevance
    } else {
        UniteLegaleSortField::DateCreation
    });
    let direction = params.direction.unwrap_or(match sort {
        // Export order follows the primary key.
        UniteLegaleSortField::Siren => SortDirection::Asc,
        _ => SortDirection::Desc,
    });

    let facets =
        search::requested_facets(params.facette.as_deref(), search::UNITE_LEGALE_FACET_FIELDS);

    let parsed = match q {
        Some(q) => Some(search::parse_query(connection, q, search::SOURCE_UNITE_LEGALE).await?),
        None => None,
    };

    let mut plan = SearchPlan {
        text: match parsed.as_ref().and_then(|parsed| parsed.tsquery.as_deref()) {
            Some(tsquery) => search::TextMatch::FullText(tsquery),
            None => search::TextMatch::None,
        },
        facets: &facets,
        cursor: cursor.as_deref(),
        sort,
        direction,
        limit,
        offset,
    };

    let mut output = execute(connection, params, &plan).await?;

    // Covers what lexicon correction cannot: transpositions and infix matches.
    // Restricted to the first page — an empty page further in means the caller
    // paged past the end, not that the search found nothing.
    if output.results.is_empty()
        && offset == 0
        && let Some(q) = q
    {
        info!(
            target: "sirene::search",
            source = search::SOURCE_UNITE_LEGALE,
            query_length = q.chars().count(),
            "repli trigramme declenche"
        );

        plan.text = search::TextMatch::Trigram(q);
        output = execute(connection, params, &plan).await?;
    }

    if output.results.is_empty() {
        output.suggestion = parsed.and_then(|parsed| parsed.suggestion);
    }

    Ok(output)
}

async fn execute(
    connection: &mut Connection,
    params: &UniteLegaleSearchParams,
    plan: &SearchPlan<'_>,
) -> Result<UniteLegaleSearchOutput, Error> {
    let mut binder = search::Binder::new();
    let mut conditions: Vec<String> = Vec::new();

    let columns = search::UNITE_LEGALE_SEARCH_COLUMNS;
    let score = match plan.text {
        search::TextMatch::None => "NULL::real".to_string(),
        search::TextMatch::FullText(tsquery) => {
            let vector = search::search_vector(Some("u"), columns);
            let placeholder = binder.push(search::Bind::TsQuery(tsquery.to_string()));
            conditions.push(format!("{vector} @@ {placeholder}::tsquery"));
            format!("ts_rank_cd({vector}, {placeholder}::tsquery)")
        }
        search::TextMatch::Trigram(q) => {
            let trigram = search::search_trigram(Some("u"), columns);
            let placeholder = binder.text(q);
            let needle = format!("lower(public.immutable_unaccent({placeholder}))");
            conditions.push(format!("{needle} <% {trigram}"));
            format!("word_similarity({needle}, {trigram})")
        }
    };

    if let Some(value) = params.etat_administratif {
        let placeholder = binder.text(match value {
            common::EtatAdministratif::A => "A",
            common::EtatAdministratif::F => "F",
        });
        conditions.push(format!("u.etat_administratif = {placeholder}"));
    }

    binder.filter_in(
        &mut conditions,
        "u.activite_principale",
        params.activite_principale.as_deref(),
    );
    binder.filter_in(
        &mut conditions,
        "u.categorie_juridique",
        params.categorie_juridique.as_deref(),
    );
    binder.filter_in(
        &mut conditions,
        "u.categorie_entreprise",
        params.categorie_entreprise.as_deref(),
    );

    binder.filter_not_in(
        &mut conditions,
        "u.activite_principale",
        params.activite_principale_not.as_deref(),
    );
    binder.filter_not_in(
        &mut conditions,
        "u.categorie_juridique",
        params.categorie_juridique_not.as_deref(),
    );
    binder.filter_not_in(
        &mut conditions,
        "u.categorie_entreprise",
        params.categorie_entreprise_not.as_deref(),
    );

    // Exact equality kept for backward compatibility; `_min` / `_max` cover the
    // same need usably.
    if let Some(value) = params.date_creation {
        let placeholder = binder.push(search::Bind::Date(value));
        conditions.push(format!("u.date_creation = {placeholder}"));
    }
    if let Some(value) = params.date_debut {
        let placeholder = binder.push(search::Bind::Date(value));
        conditions.push(format!("u.date_debut = {placeholder}"));
    }

    binder.range(
        &mut conditions,
        "u.date_creation",
        params.date_creation_min,
        params.date_creation_max,
    );
    binder.range(
        &mut conditions,
        "u.date_debut",
        params.date_debut_min,
        params.date_debut_max,
    );

    // Keyset resume: a plain bound on the primary key, served by its index. The
    // cursor was already validated as a digit string.
    if let Some(cursor) = plan.cursor {
        binder.keyset(
            &mut conditions,
            "u.siren",
            matches!(plan.direction, SortDirection::Asc),
            cursor,
        );
    }

    let dir = match plan.direction {
        SortDirection::Asc => "ASC",
        SortDirection::Desc => "DESC",
    };
    let order_by = match plan.sort {
        UniteLegaleSortField::Relevance => format!("score {dir}"),
        UniteLegaleSortField::DateCreation => format!("u.date_creation {dir} NULLS LAST"),
        UniteLegaleSortField::DateDebut => format!("u.date_debut {dir} NULLS LAST"),
        UniteLegaleSortField::Siren => format!("u.siren {dir}"),
    };

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let sql = format!(
        "SELECT u.siren, u.etat_administratif, u.date_creation, u.denomination, \
         u.denomination_usuelle_1, u.denomination_usuelle_2, u.denomination_usuelle_3, \
         u.activite_principale, u.categorie_juridique, u.categorie_entreprise, \
         {score} AS score \
         FROM unite_legale u {where_clause} ORDER BY {order_by} LIMIT {} OFFSET {}",
        plan.limit, plan.offset
    );

    let results = binder
        .apply(sql_query(sql).into_boxed())
        .load::<UniteLegaleSearchResult>(connection)
        .await
        .map_err(|error| -> Error { error.into() })?;

    let (total, total_capped) =
        search::capped_total(connection, &binder, "unite_legale", "u", &where_clause).await;
    let facettes = search::compute_facets(
        connection,
        &binder,
        "unite_legale",
        "u",
        &where_clause,
        plan.facets,
    )
    .await;

    // A full page suggests more to come. Only primary-key order is total and
    // stable enough to resume from.
    let next_cursor = match plan.sort {
        UniteLegaleSortField::Siren if results.len() as i64 == plan.limit => results
            .last()
            .map(|last| search::encode_cursor(&last.siren)),
        _ => None,
    };

    Ok(UniteLegaleSearchOutput {
        results,
        total,
        total_capped,
        limit: plan.limit,
        offset: plan.offset,
        sort: plan.sort,
        direction: plan.direction,
        suggestion: None,
        facettes,
        next_cursor,
    })
}

pub struct UniteLegaleModel {}

#[async_trait]
impl UpdatableModel for UniteLegaleModel {
    async fn count(&self, connectors: &Connectors) -> Result<i64, UpdatableError> {
        let mut connection = connectors.local.pool.get().await?;
        dsl::unite_legale
            .select(diesel::dsl::count(dsl::siren))
            .first::<i64>(&mut connection)
            .await
            .map_err(|error| error.into())
    }

    async fn count_staging(&self, connectors: &Connectors) -> Result<i64, UpdatableError> {
        use super::schema::unite_legale_staging::dsl;

        let mut connection = connectors.local.pool.get().await?;
        dsl::unite_legale_staging
            .select(diesel::dsl::count(dsl::siren))
            .first::<i64>(&mut connection)
            .await
            .map_err(|error| error.into())
    }

    fn insert_remote_file_in_staging(
        &self,
        connectors: &Connectors,
        remote_file: RemoteFile,
    ) -> Result<bool, UpdatableError> {
        use super::schema::unite_legale_staging::dsl;
        use diesel::Connection as _;
        use diesel::ExecuteCopyFromDsl as SyncExecuteCopy;
        use diesel::RunQueryDsl as SyncRunQueryDsl;

        tokio::task::block_in_place(|| {
            let mut connection =
                diesel::pg::PgConnection::establish(&connectors.local.database_url)
                    .map_err(|_| UpdatableError::SyncConnectionFailed)?;

            super::common::load_staging_without_indexes(
                &mut connection,
                "unite_legale_staging",
                |connection| {
                    SyncRunQueryDsl::execute(
                        diesel::sql_query("TRUNCATE unite_legale_staging"),
                        connection,
                    )
                    .map_err(|e| UpdatableError::Database { source: e })?;

                    let copy_query = diesel::copy_from(dsl::unite_legale_staging)
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
                sql_query("ALTER TABLE unite_legale RENAME TO unite_legale_temp")
                    .execute(conn)
                    .await?;
                sql_query("ALTER TABLE unite_legale_staging RENAME TO unite_legale")
                    .execute(conn)
                    .await?;
                sql_query("ALTER TABLE unite_legale_temp RENAME TO unite_legale_staging")
                    .execute(conn)
                    .await?;
                sql_query("TRUNCATE unite_legale_staging")
                    .execute(conn)
                    .await?;
                sql_query(
                    r#"
                    UPDATE group_metadata
                    SET last_imported_timestamp = staging_imported_timestamp
                    WHERE group_type = 'unites_legales'
                    "#,
                )
                .execute(conn)
                .await?;
                sql_query(
                    r#"
                    UPDATE group_metadata
                    SET staging_imported_timestamp = NULL
                    WHERE group_type = 'unites_legales'
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

        Ok(insee.get_total_unites_legales(start_timestamp).await?)
    }

    async fn get_last_insee_synced_timestamp(
        &self,
        connectors: &Connectors,
    ) -> Result<Option<NaiveDateTime>, UpdatableError> {
        let mut connection = connectors.local.pool.get().await?;
        dsl::unite_legale
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

        let (next_cursor, unites_legales) = insee
            .get_daily_unites_legales(start_timestamp, cursor)
            .await?;

        let mut connection = connectors.local.pool.get().await?;

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
            .execute(&mut connection)
            .await?;

        Ok((next_cursor, updated_count))
    }

    fn search_source(&self) -> Option<&'static str> {
        Some(search::SOURCE_UNITE_LEGALE)
    }
}
