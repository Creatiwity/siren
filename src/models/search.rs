//! Shared building blocks for the `etablissement` and `unite_legale` searches.
//!
//! Matching uses native PostgreSQL full-text search. Typo tolerance corrects the
//! *query* against the corpus lexicon rather than fuzzy-matching the corpus, and
//! a trigram pass runs only when full-text search returns nothing.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use diesel::pg::Pg;
use diesel::query_builder::{BoxedSqlQuery, SqlQuery};
use diesel::sql_types::{Array, BigInt, Bool, Date, Float8, Nullable, Text};
use diesel::{QueryableByName, sql_query};
use diesel_async::RunQueryDsl;
use serde::Serialize;
use utoipa::ToSchema;

use crate::connectors::local::Connection;

/// Below this, no index structure is selective enough and correction is
/// meaningless: every three-letter word is one edit away from every other.
pub const MIN_QUERY_LENGTH: usize = 3;

/// Counting stops here rather than walking every match; `total_capped` reports
/// whether the ceiling was hit.
pub const SEARCH_TOTAL_CAP: i64 = 10_000;

pub const FACET_VALUES_LIMIT: i64 = 20;

pub const SOURCE_ETABLISSEMENT: &str = "etablissement";
pub const SOURCE_UNITE_LEGALE: &str = "unite_legale";

pub const ETABLISSEMENT_SEARCH_COLUMNS: &[&str] = &[
    "denomination_usuelle",
    "enseigne_1",
    "enseigne_2",
    "enseigne_3",
];
pub const UNITE_LEGALE_SEARCH_COLUMNS: &[&str] = &[
    "denomination",
    "denomination_usuelle_1",
    "denomination_usuelle_2",
    "denomination_usuelle_3",
];

/// Facetable fields. These names are interpolated into SQL, so user input must
/// never reach a query without passing through this whitelist.
pub const ETABLISSEMENT_FACET_FIELDS: &[&str] = &[
    "etat_administratif",
    "code_postal",
    "code_commune",
    "activite_principale",
    "etablissement_siege",
];
pub const UNITE_LEGALE_FACET_FIELDS: &[&str] = &[
    "etat_administratif",
    "activite_principale",
    "categorie_juridique",
    "categorie_entreprise",
];

/// How the text query is matched.
pub enum TextMatch<'a> {
    None,
    /// Native full-text search. Nominal path.
    FullText(&'a str),
    /// Trigram fallback, only when full-text search returns nothing.
    Trigram(&'a str),
}

/// Must stay byte-identical to the expression indexes created by migration
/// `2026-09-15-120000_search_fts_commune`. Any divergence silently disables the
/// GIN indexes — no error, just sequential scans. Locked by the tests below.
///
/// `alias` is `None` for the DDL form (bare columns), `Some("e")` for queries.
fn search_text(alias: Option<&str>, columns: &[&str]) -> String {
    let prefix = alias.map(|a| format!("{a}.")).unwrap_or_default();
    let parts: Vec<String> = columns
        .iter()
        .map(|column| format!("coalesce({prefix}{column}, '')"))
        .collect();

    format!("public.immutable_unaccent({})", parts.join(" || ' ' || "))
}

pub fn search_vector(alias: Option<&str>, columns: &[&str]) -> String {
    format!(
        "to_tsvector('french'::regconfig, {})",
        search_text(alias, columns)
    )
}

pub fn search_trigram(alias: Option<&str>, columns: &[&str]) -> String {
    format!("lower({})", search_text(alias, columns))
}

#[derive(Debug, QueryableByName)]
pub struct ParsedQuery {
    /// Text form, to be cast back with `::tsquery`. `None` when the input holds
    /// no usable word.
    #[diesel(sql_type = Nullable<Text>)]
    pub tsquery: Option<String>,
    /// Set only when a word was judged misspelled. Exposed to clients only when
    /// the search returns nothing.
    #[diesel(sql_type = Nullable<Text>)]
    pub suggestion: Option<String>,
}

/// Builds the augmented `tsquery`: a suspect word is completed with
/// `| correction`, never replaced, so an exact match can never be lost.
pub async fn parse_query(
    connection: &mut Connection,
    q: &str,
    source: &str,
) -> Result<ParsedQuery, diesel::result::Error> {
    sql_query("SELECT tsq::text AS tsquery, suggestion FROM public.search_query($1, $2)")
        .bind::<Text, _>(q.to_string())
        .bind::<Text, _>(source.to_string())
        .get_result::<ParsedQuery>(connection)
        .await
}

pub async fn resolve_commune(
    connection: &mut Connection,
    commune: &str,
) -> Result<Vec<String>, diesel::result::Error> {
    #[derive(QueryableByName)]
    struct Codes {
        #[diesel(sql_type = Array<Text>)]
        codes: Vec<String>,
    }

    sql_query("SELECT public.commune_codes($1) AS codes")
        .bind::<Text, _>(commune.to_string())
        .get_result::<Codes>(connection)
        .await
        .map(|row| row.codes)
}

#[derive(QueryableByName)]
pub struct RowCount {
    #[diesel(sql_type = BigInt)]
    pub count: i64,
}

/// Higher than the offset limit: a keyset page costs a bounded index walk,
/// where `OFFSET n` always pays `n`.
pub const CURSOR_LIMIT_MAX: i64 = 1_000;

/// Holds the last primary key returned, nothing else: a cursor is only valid
/// when replayed with the same search parameters.
pub fn encode_cursor(primary_key: &str) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(primary_key)
}

/// Returns `None` on an unreadable cursor.
pub fn decode_cursor(cursor: &str) -> Option<String> {
    use base64::Engine as _;
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(cursor)
        .ok()?;
    let value = String::from_utf8(decoded).ok()?;

    // A SIRET or SIREN is digits only. Anything else is forged or corrupted and
    // is rejected rather than fed into a comparison.
    (!value.is_empty()
        && value.len() <= 14
        && value.chars().all(|character| character.is_ascii_digit()))
    .then_some(value)
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FacetValue {
    pub valeur: String,
    pub nombre: i64,
}

pub type Facets = BTreeMap<String, Vec<FacetValue>>;

#[derive(QueryableByName)]
struct FacetRow {
    #[diesel(sql_type = Text)]
    champ: String,
    #[diesel(sql_type = Text)]
    valeur: String,
    #[diesel(sql_type = BigInt)]
    nombre: i64,
}

pub fn split_values(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

/// Whitelisted requested fields, deduplicated, in request order.
pub fn requested_facets(raw: Option<&str>, allowed: &[&str]) -> Vec<String> {
    let Some(raw) = raw else {
        return Vec::new();
    };

    let mut fields: Vec<String> = Vec::new();
    for field in split_values(raw) {
        if allowed.contains(&field.as_str()) && !fields.contains(&field) {
            fields.push(field);
        }
    }
    fields
}

/// Rejects a text query too short to be served by any index. The model degrades
/// gracefully instead; the HTTP layer prefers to say so.
pub fn check_query_length(q: Option<&str>) -> Result<(), String> {
    match q.map(str::trim) {
        Some(q) if q.chars().count() < MIN_QUERY_LENGTH => Err(format!(
            "q must be at least {MIN_QUERY_LENGTH} characters long"
        )),
        _ => Ok(()),
    }
}

/// Rejects a facet field outside the whitelist: ignoring it would leave the
/// client waiting for counts that never come.
pub fn check_facets(raw: Option<&str>, allowed: &[&str]) -> Result<(), String> {
    let unknown = unknown_facets(raw, allowed);
    if unknown.is_empty() {
        return Ok(());
    }
    Err(format!(
        "unknown facette field(s): {}. Allowed: {}",
        unknown.join(", "),
        allowed.join(", ")
    ))
}

/// Rejects a cursor that cannot be honoured.
///
/// Keyset resume only makes sense on primary-key order. On relevance or distance
/// it would gain nothing — score and distance are recomputed row by row, there is
/// no walk to shorten. On a date it would need a composite index: the densest tie
/// group holds 583,632 rows and resuming inside it costs 38.8 s without one.
pub fn check_cursor(
    cursor: Option<&str>,
    primary_key_sort: bool,
    sort_name: &str,
    offset: Option<i64>,
) -> Result<(), String> {
    let Some(cursor) = cursor else {
        return Ok(());
    };
    if !primary_key_sort {
        return Err(format!("cursor requires sort={sort_name}"));
    }
    if offset.is_some() {
        return Err("cursor and offset are mutually exclusive".to_string());
    }
    // Ignoring an unreadable cursor would return the first page, and a client
    // looping on `next_cursor` would never notice it is going in circles.
    if decode_cursor(cursor.trim()).is_none() {
        return Err("cursor is malformed".to_string());
    }
    Ok(())
}

/// Requested fields outside the whitelist, so they can be refused rather than
/// silently dropped.
pub fn unknown_facets(raw: Option<&str>, allowed: &[&str]) -> Vec<String> {
    raw.map(|raw| {
        split_values(raw)
            .into_iter()
            .filter(|field| !allowed.contains(&field.as_str()))
            .collect()
    })
    .unwrap_or_default()
}

#[derive(Debug, Clone)]
pub enum Bind {
    Text(String),
    Bool(bool),
    Float8(f64),
    Date(NaiveDate),
    TsQuery(String),
    TextArray(Vec<String>),
}

/// Collects bound values so the same ordering is replayed across the result,
/// count and facet queries without keeping parallel vectors in sync by hand.
#[derive(Debug, Default)]
pub struct Binder {
    binds: Vec<Bind>,
}

impl Binder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, bind: Bind) -> String {
        self.binds.push(bind);
        format!("${}", self.binds.len())
    }

    pub fn text(&mut self, value: impl Into<String>) -> String {
        self.push(Bind::Text(value.into()))
    }

    pub fn any_of(&mut self, column: &str, values: Vec<String>) -> String {
        let placeholder = self.push(Bind::TextArray(values));
        format!("{column} = ANY({placeholder})")
    }

    /// Rows whose column is NULL are kept: "not 62.01Z" includes "unknown
    /// activity".
    pub fn none_of(&mut self, column: &str, values: Vec<String>) -> String {
        let placeholder = self.push(Bind::TextArray(values));
        format!("({column} IS NULL OR {column} <> ALL({placeholder}))")
    }

    /// Adds an inclusion filter, if the parameter carries any value.
    pub fn filter_in(&mut self, conditions: &mut Vec<String>, column: &str, raw: Option<&str>) {
        let values = raw.map(split_values).unwrap_or_default();
        if !values.is_empty() {
            conditions.push(self.any_of(column, values));
        }
    }

    /// Adds an exclusion filter, if the parameter carries any value.
    pub fn filter_not_in(&mut self, conditions: &mut Vec<String>, column: &str, raw: Option<&str>) {
        let values = raw.map(split_values).unwrap_or_default();
        if !values.is_empty() {
            conditions.push(self.none_of(column, values));
        }
    }

    /// Adds the bounds that are present, each one optional.
    pub fn range(
        &mut self,
        conditions: &mut Vec<String>,
        column: &str,
        min: Option<NaiveDate>,
        max: Option<NaiveDate>,
    ) {
        if let Some(min) = min {
            let placeholder = self.push(Bind::Date(min));
            conditions.push(format!("{column} >= {placeholder}"));
        }
        if let Some(max) = max {
            let placeholder = self.push(Bind::Date(max));
            conditions.push(format!("{column} <= {placeholder}"));
        }
    }

    /// Adds a keyset bound on the primary key.
    pub fn keyset(
        &mut self,
        conditions: &mut Vec<String>,
        column: &str,
        ascending: bool,
        cursor: &str,
    ) {
        let placeholder = self.text(cursor);
        let comparison = if ascending { ">" } else { "<" };
        conditions.push(format!("{column} {comparison} {placeholder}"));
    }

    /// Applies the values in registration order.
    pub fn apply<'a>(
        &self,
        query: BoxedSqlQuery<'a, Pg, SqlQuery>,
    ) -> BoxedSqlQuery<'a, Pg, SqlQuery> {
        self.binds.iter().fold(query, |query, bind| match bind {
            Bind::Text(value) => query.bind::<Text, _>(value.clone()),
            Bind::Bool(value) => query.bind::<Bool, _>(*value),
            Bind::Float8(value) => query.bind::<Float8, _>(*value),
            Bind::Date(value) => query.bind::<Date, _>(*value),
            Bind::TsQuery(value) => query.bind::<Text, _>(value.clone()),
            Bind::TextArray(values) => query.bind::<Array<Text>, _>(values.clone()),
        })
    }
}

/// Returns `(total, capped)`. Ordering is irrelevant here, so the plain form
/// works even when the main query goes through a lateral join.
pub async fn capped_total(
    connection: &mut Connection,
    binder: &Binder,
    table: &str,
    alias: &str,
    where_clause: &str,
) -> (i64, bool) {
    let sql = format!(
        "SELECT count(*) AS count FROM (SELECT 1 FROM {table} {alias} {where_clause} LIMIT {}) _sub",
        SEARCH_TOTAL_CAP + 1
    );

    binder
        .apply(sql_query(sql).into_boxed())
        .get_result::<RowCount>(connection)
        .await
        .map(|row| {
            let capped = row.count > SEARCH_TOTAL_CAP;
            (row.count.min(SEARCH_TOTAL_CAP), capped)
        })
        .unwrap_or((0, false))
}

/// Computed over the same bounded subset as the count, in a single round trip
/// whatever the number of fields. Field names must come from
/// [`requested_facets`], hence from the whitelist.
pub async fn compute_facets(
    connection: &mut Connection,
    binder: &Binder,
    table: &str,
    alias: &str,
    where_clause: &str,
    fields: &[String],
) -> Facets {
    if fields.is_empty() {
        return Facets::new();
    }

    let projection: Vec<String> = fields
        .iter()
        .map(|field| format!("{alias}.{field}"))
        .collect();

    let counts: Vec<String> = fields
        .iter()
        .map(|field| {
            format!(
                "(SELECT '{field}' AS champ, {field}::text AS valeur, count(*) AS nombre \
                  FROM candidats WHERE {field} IS NOT NULL GROUP BY 2 \
                  ORDER BY nombre DESC, valeur LIMIT {FACET_VALUES_LIMIT})"
            )
        })
        .collect();

    let sql = format!(
        "WITH candidats AS (SELECT {} FROM {table} {alias} {where_clause} LIMIT {}) \
         SELECT champ, valeur, nombre FROM ({}) comptes ORDER BY champ, nombre DESC, valeur",
        projection.join(", "),
        SEARCH_TOTAL_CAP,
        counts.join(" UNION ALL ")
    );

    let rows = binder
        .apply(sql_query(sql).into_boxed())
        .load::<FacetRow>(connection)
        .await
        .unwrap_or_default();

    let mut facets = Facets::new();
    for field in fields {
        facets.insert(field.clone(), Vec::new());
    }
    for row in rows {
        facets.entry(row.champ).or_default().push(FacetValue {
            valeur: row.valeur,
            nombre: row.nombre,
        });
    }
    facets
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drops whitespace outside quoted literals, so the SQL formatting is
    /// irrelevant but the `' '` separator between columns still counts.
    ///
    /// Removing whitespace unconditionally would erase that separator too, and
    /// the comparison would then accept a migration concatenating the columns
    /// with nothing in between.
    /// Comment lines are dropped first: an apostrophe in prose would otherwise
    /// flip the quoting state and desynchronise everything after it.
    fn normalized(value: &str) -> String {
        let mut out = String::with_capacity(value.len());
        let mut in_literal = false;

        for line in value.lines() {
            if line.trim_start().starts_with("--") {
                continue;
            }
            for character in line.chars() {
                if character == '\'' {
                    in_literal = !in_literal;
                }
                if in_literal || !character.is_whitespace() {
                    out.push(character);
                }
            }
        }
        out
    }

    const SEARCH_MIGRATION: &str =
        include_str!("../../migrations/2026-09-15-120000_search_fts_commune/up.sql");

    #[test]
    fn expressions_fts_identiques_au_ddl() {
        let migration = normalized(SEARCH_MIGRATION);

        for columns in [ETABLISSEMENT_SEARCH_COLUMNS, UNITE_LEGALE_SEARCH_COLUMNS] {
            let expression = normalized(&search_vector(None, columns));
            assert!(
                migration.contains(&expression),
                "tsvector expression missing from the migration: {expression}"
            );
        }
    }

    #[test]
    fn expressions_trigramme_identiques_au_ddl() {
        let migration = normalized(SEARCH_MIGRATION);

        for columns in [ETABLISSEMENT_SEARCH_COLUMNS, UNITE_LEGALE_SEARCH_COLUMNS] {
            let expression = normalized(&search_trigram(None, columns));
            assert!(
                migration.contains(&expression),
                "trigram expression missing from the migration: {expression}"
            );
        }
    }

    /// The same column list also feeds `search_refresh_full` and
    /// `search_refresh_incremental`, which build the correction lexicon. Those
    /// two copies were guarded by nothing: a divergence there would silently
    /// build the lexicon from different columns than the index, and typo
    /// correction would target the wrong vocabulary.
    #[test]
    fn colonnes_identiques_dans_les_fonctions_de_rafraichissement() {
        // Scoped to the two refresh functions, so the two index definitions
        // above them are not what makes this pass.
        let refreshes = normalized(
            &SEARCH_MIGRATION[SEARCH_MIGRATION
                .find("FUNCTION public.search_refresh_full")
                .expect("search_refresh_full missing from the migration")..],
        );

        for columns in [ETABLISSEMENT_SEARCH_COLUMNS, UNITE_LEGALE_SEARCH_COLUMNS] {
            // The refresh functions concatenate the same columns, applying
            // `immutable_unaccent` themselves.
            let concatenation = normalized(
                &columns
                    .iter()
                    .map(|column| format!("coalesce({column},'')"))
                    .collect::<Vec<_>>()
                    .join("||' '||"),
            );
            assert_eq!(
                refreshes.matches(&concatenation).count(),
                2,
                "both refresh functions must build the lexicon from: {concatenation}"
            );
        }
    }

    #[test]
    fn alias_prefixe_les_colonnes() {
        let expression = search_vector(Some("e"), ETABLISSEMENT_SEARCH_COLUMNS);
        assert!(expression.contains("coalesce(e.denomination_usuelle, '')"));
        assert!(!expression.contains("coalesce(denomination_usuelle, '')"));
    }

    #[test]
    fn decoupage_multi_valeurs() {
        assert_eq!(split_values("a, b ,c"), vec!["a", "b", "c"]);
        assert_eq!(split_values(" , ,"), Vec::<String>::new());
        assert_eq!(split_values("62.01Z"), vec!["62.01Z"]);
    }

    #[test]
    fn facettes_filtrees_par_liste_blanche() {
        let allowed = ETABLISSEMENT_FACET_FIELDS;
        assert_eq!(
            requested_facets(Some("code_commune,activite_principale"), allowed),
            vec!["code_commune", "activite_principale"]
        );
        assert_eq!(
            requested_facets(Some("code_postal,code_postal"), allowed),
            vec!["code_postal"]
        );
        assert!(requested_facets(Some("siret; DROP TABLE etablissement"), allowed).is_empty());
        assert_eq!(
            unknown_facets(Some("code_postal,inconnu"), allowed),
            vec!["inconnu"]
        );
    }

    #[test]
    fn curseur_aller_retour() {
        let cursor = encode_cursor("12345678900011");
        assert_ne!(cursor, "12345678900011", "le curseur doit etre opaque");
        assert_eq!(decode_cursor(&cursor).as_deref(), Some("12345678900011"));
    }

    #[test]
    fn curseur_forge_refuse() {
        for forged in ["', 'x", "0' OR '1'='1", "", "abc", "123456789000111"] {
            assert_eq!(
                decode_cursor(&encode_cursor(forged)),
                None,
                "curseur accepte a tort : {forged:?}"
            );
        }
        assert_eq!(decode_cursor("pas du base64 !!"), None);
    }

    #[test]
    fn negation_conserve_les_valeurs_nulles() {
        let mut binder = Binder::new();
        let condition = binder.none_of("e.activite_principale", vec!["62.01Z".to_string()]);
        assert_eq!(
            condition,
            "(e.activite_principale IS NULL OR e.activite_principale <> ALL($1))"
        );
    }
}
