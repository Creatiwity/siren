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
use diesel::sql_types::{Text, Timestamp};
use diesel_async::{AsyncConnection, RunQueryDsl};
use error::Error;

pub async fn get(connection: &mut Connection, siren: &str) -> Result<UniteLegale, Error> {
    dsl::unite_legale
        .find(siren)
        .select(UniteLegale::as_select())
        .first::<UniteLegale>(connection)
        .await
        .map_err(|error| error.into())
}

/// Mode de correspondance textuelle applique a la requete.
enum TextMatch<'a> {
    None,
    /// FTS natif : `tsvector @@ tsquery`. Chemin nominal.
    FullText(&'a str),
    /// Repli trigramme, declenche uniquement quand le FTS ne ramene rien.
    Trigram(&'a str),
}

pub async fn search(
    connection: &mut Connection,
    params: &UniteLegaleSearchParams,
) -> Result<UniteLegaleSearchOutput, Error> {
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let offset = params.offset.unwrap_or(0).clamp(0, 10_000);

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
    let direction = params.direction.unwrap_or(SortDirection::Desc);

    // Analyse de la saisie : tsquery augmentee (chaque mot suspect est complete
    // par `| correction`, jamais remplace) et reformulation proposee.
    let parsed = match q {
        Some(q) => Some(search::parse_query(connection, q, search::SOURCE_UNITE_LEGALE).await?),
        None => None,
    };

    let text = match parsed.as_ref().and_then(|parsed| parsed.tsquery.as_deref()) {
        Some(tsquery) => TextMatch::FullText(tsquery),
        None => TextMatch::None,
    };

    let mut output = execute(connection, params, &text, sort, direction, limit, offset).await?;

    // Repli trigramme : couvre ce que la correction par lexique ne rattrape pas
    // (transpositions, correspondance infixe). Jamais sur le chemin chaud.
    if output.results.is_empty()
        && let Some(q) = q
    {
        output = execute(
            connection,
            params,
            &TextMatch::Trigram(q),
            sort,
            direction,
            limit,
            offset,
        )
        .await?;
    }

    if output.results.is_empty() {
        output.suggestion = parsed.and_then(|parsed| parsed.suggestion);
    }

    Ok(output)
}

async fn execute(
    connection: &mut Connection,
    params: &UniteLegaleSearchParams,
    text: &TextMatch<'_>,
    sort: UniteLegaleSortField,
    direction: SortDirection,
    limit: i64,
    offset: i64,
) -> Result<UniteLegaleSearchOutput, Error> {
    let mut binder = search::Binder::new();

    let vector = search::search_vector("u", search::UNITE_LEGALE_SEARCH_COLUMNS);
    let trigram = search::search_trigram("u", search::UNITE_LEGALE_SEARCH_COLUMNS);

    let mut conditions: Vec<String> = Vec::new();

    let score = match text {
        TextMatch::None => "NULL::real".to_string(),
        TextMatch::FullText(tsquery) => {
            let placeholder = binder.push(search::Bind::TsQuery(tsquery.to_string()));
            conditions.push(format!("{vector} @@ {placeholder}::tsquery"));
            format!("ts_rank_cd({vector}, {placeholder}::tsquery)")
        }
        TextMatch::Trigram(q) => {
            let placeholder = binder.text(*q);
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
    if let Some(value) = params.activite_principale.as_deref() {
        let placeholder = binder.text(value);
        conditions.push(format!("u.activite_principale = {placeholder}"));
    }
    if let Some(value) = params.categorie_juridique.as_deref() {
        let placeholder = binder.text(value);
        conditions.push(format!("u.categorie_juridique = {placeholder}"));
    }
    if let Some(value) = params.categorie_entreprise.as_deref() {
        let placeholder = binder.text(value);
        conditions.push(format!("u.categorie_entreprise = {placeholder}"));
    }
    if let Some(value) = params.date_creation {
        let placeholder = binder.push(search::Bind::Date(value));
        conditions.push(format!("u.date_creation = {placeholder}"));
    }
    if let Some(value) = params.date_debut {
        let placeholder = binder.push(search::Bind::Date(value));
        conditions.push(format!("u.date_debut = {placeholder}"));
    }

    let dir = match direction {
        SortDirection::Asc => "ASC",
        SortDirection::Desc => "DESC",
    };
    let order_by = match sort {
        UniteLegaleSortField::Relevance => format!("score {dir}"),
        UniteLegaleSortField::DateCreation => format!("u.date_creation {dir} NULLS LAST"),
        UniteLegaleSortField::DateDebut => format!("u.date_debut {dir} NULLS LAST"),
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
         FROM unite_legale u {where_clause} ORDER BY {order_by} LIMIT {limit} OFFSET {offset}"
    );

    let results = binder
        .apply(sql_query(sql).into_boxed())
        .load::<UniteLegaleSearchResult>(connection)
        .await
        .map_err(|error| -> Error { error.into() })?;

    let count_sql = format!(
        "SELECT count(*) AS count FROM (SELECT 1 FROM unite_legale u {where_clause} LIMIT {}) _sub",
        search::SEARCH_TOTAL_CAP + 1
    );

    let total = binder
        .apply(sql_query(count_sql).into_boxed())
        .get_result::<search::RowCount>(connection)
        .await
        .map(|row| row.count.min(search::SEARCH_TOTAL_CAP))
        .unwrap_or(0);

    Ok(UniteLegaleSearchOutput {
        results,
        total,
        limit,
        offset,
        sort,
        direction,
        suggestion: None,
    })
}

pub struct UniteLegaleModel {}

#[async_trait]
impl UpdatableModel for UniteLegaleModel {
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

            SyncRunQueryDsl::execute(
                diesel::sql_query("TRUNCATE unite_legale_staging"),
                &mut connection,
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
            SyncExecuteCopy::execute(copy_query, &mut connection)
                .map(|count| count > 0)
                .map_err(|e| UpdatableError::Database { source: e })
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

    async fn refresh_search_metadata(
        &self,
        connectors: &Connectors,
        since: Option<NaiveDateTime>,
    ) -> Result<(), UpdatableError> {
        let Some(source) = self.search_source() else {
            return Ok(());
        };

        let mut connection = connectors.local.pool.get().await?;

        match since {
            Some(since) => {
                sql_query("SELECT public.search_refresh_incremental($1, $2)")
                    .bind::<Text, _>(source)
                    .bind::<Timestamp, _>(since)
                    .execute(&mut connection)
                    .await?;
            }
            None => {
                sql_query("SELECT public.search_refresh_full($1)")
                    .bind::<Text, _>(source)
                    .execute(&mut connection)
                    .await?;

                // Sans statistiques fraiches, le planificateur retombe sur des
                // estimations par defaut et choisit des seq scans sur les
                // predicats GIN. Le RENAME du swap laisse la table sans stats
                // representatives : il faut les recalculer explicitement.
                sql_query("ANALYZE unite_legale")
                    .execute(&mut connection)
                    .await?;

                // La reconstruction remplace l'integralite des lignes de la
                // source et laisse autant de tuples morts derriere elle
                // (1,37 M mesures pour etablissement). VACUUM les recupere sans
                // bloquer lectures ni ecritures — contrairement a VACUUM FULL,
                // a ne jamais utiliser ici.
                sql_query("VACUUM (ANALYZE) search_lexicon")
                    .execute(&mut connection)
                    .await?;
            }
        }

        Ok(())
    }
}
