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
use diesel::sql_types::{Text, Timestamp};
use diesel_async::{AsyncConnection, RunQueryDsl};
use error::Error;

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

/// Mode de correspondance textuelle applique a la requete.
enum TextMatch<'a> {
    /// Aucun filtre texte.
    None,
    /// FTS natif : `tsvector @@ tsquery`. Chemin nominal.
    FullText(&'a str),
    /// Repli trigramme, declenche uniquement quand le FTS ne ramene rien.
    Trigram(&'a str),
}

/// Parametres resolus d'une execution de recherche.
struct SearchPlan<'a> {
    text: TextMatch<'a>,
    commune_codes: Option<&'a [String]>,
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

    let empty = |suggestion| EtablissementSearchOutput {
        results: Vec::new(),
        total: 0,
        limit,
        offset,
        sort,
        direction,
        suggestion,
    };

    // Resolution du libelle de commune en codes INSEE. Un libelle sans
    // correspondance ne peut rien donner : inutile d'interroger la grande table.
    let commune_codes = match params
        .commune
        .as_deref()
        .map(str::trim)
        .filter(|commune| !commune.is_empty())
    {
        Some(commune) => {
            let codes = search::resolve_commune(connection, commune).await?;
            if codes.is_empty() {
                return Ok(empty(None));
            }
            Some(codes)
        }
        None => None,
    };

    // Analyse de la saisie : tsquery augmentee (chaque mot suspect est complete
    // par `| correction`, jamais remplace) et reformulation proposee.
    let parsed = match q {
        Some(q) => Some(search::parse_query(connection, q, search::SOURCE_ETABLISSEMENT).await?),
        None => None,
    };

    let text = match parsed.as_ref().and_then(|parsed| parsed.tsquery.as_deref()) {
        Some(tsquery) => TextMatch::FullText(tsquery),
        None => TextMatch::None,
    };

    let mut output = execute(
        connection,
        params,
        &SearchPlan {
            text,
            commune_codes: commune_codes.as_deref(),
            sort,
            direction,
            limit,
            offset,
        },
    )
    .await?;

    // Repli trigramme : couvre ce que la correction par lexique ne rattrape pas
    // (transpositions, correspondance infixe). Il ne s'execute que sur les
    // recherches sans resultat, donc jamais sur le chemin chaud.
    if output.results.is_empty()
        && let Some(q) = q
    {
        output = execute(
            connection,
            params,
            &SearchPlan {
                text: TextMatch::Trigram(q),
                commune_codes: commune_codes.as_deref(),
                sort,
                direction,
                limit,
                offset,
            },
        )
        .await?;
    }

    // La reformulation n'est exposee que si la recherche reste vide : proposee
    // systematiquement, elle serait fausse pour ~45 % des noms rares mais
    // corrects (mesure sur le corpus).
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
    let has_geo = params.lat.is_some() && params.lng.is_some() && params.radius.is_some();

    let mut binder = search::Binder::new();

    // Point de reference geographique : declare en premier pour que l'expression
    // soit constante au moment de la planification, condition du parcours KNN
    // sur l'index GiST.
    let reference_point = has_geo.then(|| {
        let lng = binder.push(search::Bind::Float8(params.lng.unwrap_or_default()));
        let lat = binder.push(search::Bind::Float8(params.lat.unwrap_or_default()));
        format!("ST_SetSRID(ST_MakePoint({lng}, {lat}), 4326)::geography")
    });

    let vector = search::search_vector("e", search::ETABLISSEMENT_SEARCH_COLUMNS);
    let trigram = search::search_trigram("e", search::ETABLISSEMENT_SEARCH_COLUMNS);

    let mut conditions: Vec<String> = Vec::new();

    let score = match plan.text {
        TextMatch::None => "NULL::real".to_string(),
        TextMatch::FullText(tsquery) => {
            let placeholder = binder.push(search::Bind::TsQuery(tsquery.to_string()));
            conditions.push(format!("{vector} @@ {placeholder}::tsquery"));
            format!("ts_rank_cd({vector}, {placeholder}::tsquery)")
        }
        TextMatch::Trigram(q) => {
            let placeholder = binder.text(q);
            let needle = format!("lower(public.immutable_unaccent({placeholder}))");
            conditions.push(format!("{needle} <% {trigram}"));
            format!("word_similarity({needle}, {trigram})")
        }
    };

    let distance = match &reference_point {
        Some(point) => {
            let radius = binder.push(search::Bind::Float8(params.radius.unwrap_or_default()));
            conditions.push(format!("ST_DWithin(e.position, {point}, {radius})"));
            format!("ST_Distance(e.position, {point})")
        }
        None => "NULL::float8".to_string(),
    };

    // Filtre commune : la resolution du libelle est deja faite, on ne manipule
    // plus que des codes INSEE, servis par etablissement_commune_date_idx.
    let commune_placeholder = plan
        .commune_codes
        .map(|codes| binder.push(search::Bind::TextArray(codes.to_vec())));

    if let Some(placeholder) = &commune_placeholder {
        conditions.push(format!("e.code_commune = ANY({placeholder})"));
    }

    if let Some(value) = params.etat_administratif {
        let placeholder = binder.text(match value {
            common::EtatAdministratif::A => "A",
            common::EtatAdministratif::F => "F",
        });
        conditions.push(format!("e.etat_administratif = {placeholder}"));
    }
    if let Some(value) = params.code_postal.as_deref() {
        let placeholder = binder.text(value);
        conditions.push(format!("e.code_postal = {placeholder}"));
    }
    if let Some(value) = params.siren.as_deref() {
        let placeholder = binder.text(value);
        conditions.push(format!("e.siren = {placeholder}"));
    }
    if let Some(value) = params.code_commune.as_deref() {
        let placeholder = binder.text(value);
        conditions.push(format!("e.code_commune = {placeholder}"));
    }
    if let Some(value) = params.activite_principale.as_deref() {
        let placeholder = binder.text(value);
        conditions.push(format!("e.activite_principale = {placeholder}"));
    }
    if let Some(value) = params.etablissement_siege {
        let placeholder = binder.push(search::Bind::Bool(value));
        conditions.push(format!("e.etablissement_siege = {placeholder}"));
    }

    let order_by = order_by_clause(plan.sort, plan.direction, reference_point.as_deref());

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let columns = format!(
        "e.siret, e.siren, e.etat_administratif, e.date_creation, e.denomination_usuelle, \
         e.enseigne_1, e.enseigne_2, e.enseigne_3, e.code_postal, e.libelle_commune, \
         e.activite_principale, e.etablissement_siege, e.position, \
         {distance} AS meter_distance, {score} AS score"
    );

    // Filtrer par commune sans filtre texte revient a trier des millions de
    // lignes : `code_commune = ANY(...)` ne peut pas rendre l'ordre de l'index.
    // On passe alors par un top-N par commune, fusionne ensuite (1,5 ms contre
    // 22 s sur `commune=paris`).
    let sql = match (&commune_placeholder, &plan.text, has_geo) {
        (Some(placeholder), TextMatch::None, false) => {
            let inner_conditions: Vec<&String> = conditions
                .iter()
                .filter(|condition| !condition.starts_with("e.code_commune = ANY("))
                .collect();

            let mut inner = vec!["e.code_commune = codes.code_commune".to_string()];
            inner.extend(inner_conditions.into_iter().cloned());

            format!(
                "SELECT {columns} FROM unnest({placeholder}) AS codes(code_commune) \
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
            "SELECT {columns} FROM etablissement e {where_clause} \
             ORDER BY {order_by} LIMIT {} OFFSET {}",
            plan.limit, plan.offset
        ),
    };

    let results = binder
        .apply(sql_query(sql).into_boxed())
        .load::<EtablissementSearchResult>(connection)
        .await
        .map_err(|error| -> Error { error.into() })?;

    // Comptage plafonne : le tri n'est pas necessaire, la forme simple suffit
    // meme dans le cas commune-sans-texte.
    let count_sql = format!(
        "SELECT count(*) AS count FROM (SELECT 1 FROM etablissement e {where_clause} LIMIT {}) _sub",
        search::SEARCH_TOTAL_CAP + 1
    );

    let total = binder
        .apply(sql_query(count_sql).into_boxed())
        .get_result::<search::RowCount>(connection)
        .await
        .map(|row| row.count.min(search::SEARCH_TOTAL_CAP))
        .unwrap_or(0);

    Ok(EtablissementSearchOutput {
        results,
        total,
        limit: plan.limit,
        offset: plan.offset,
        sort: plan.sort,
        direction: plan.direction,
        suggestion: None,
    })
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

            SyncRunQueryDsl::execute(
                diesel::sql_query("TRUNCATE etablissement_staging"),
                &mut connection,
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
            SyncExecuteCopy::execute(copy_query, &mut connection)
                .map(|count| count > 0)
                .map_err(|e| UpdatableError::Database { source: e })
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
                sql_query("ANALYZE etablissement")
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
