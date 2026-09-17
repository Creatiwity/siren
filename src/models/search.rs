//! Briques communes aux recherches `etablissement` et `unite_legale`.
//!
//! Le matching repose sur le FTS natif PostgreSQL (`tsvector`/`tsquery`), la
//! tolerance aux fautes sur une correction de la *requete* via le lexique du
//! corpus (`search_lexicon`), et un repli trigramme declenche uniquement quand
//! le FTS ne ramene rien.

use chrono::NaiveDate;
use diesel::pg::Pg;
use diesel::query_builder::{BoxedSqlQuery, SqlQuery};
use diesel::sql_types::{BigInt, Bool, Date, Float8, Nullable, Text};
use diesel::{QueryableByName, sql_query};
use diesel_async::RunQueryDsl;

use crate::connectors::local::Connection;

/// Longueur minimale d'une requete texte. En dessous, le trigramme comme le
/// FTS renvoient des volumes ingerables et la correction n'a aucun sens
/// (tous les mots de trois lettres sont a distance 1 les uns des autres).
pub const MIN_QUERY_LENGTH: usize = 3;

/// Plafond du comptage total : au dela, `total` est renvoye plafonne plutot
/// que de parcourir l'integralite des correspondances.
pub const SEARCH_TOTAL_CAP: i64 = 10_000;

/// Identifiants de source utilises par `search_lexicon.source` et par les
/// fonctions SQL `search_query` / `search_refresh_*`.
pub const SOURCE_ETABLISSEMENT: &str = "etablissement";
pub const SOURCE_UNITE_LEGALE: &str = "unite_legale";

/// Colonnes composant le texte recherchable de chaque table.
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

/// Concatenation normalisee des colonnes de nom.
///
/// ATTENTION : cette expression doit rester strictement identique a celle des
/// index d'expression crees par la migration `2026-09-15-120000_search_fts_commune`.
/// Toute divergence (un espace, un `coalesce` en plus) rend les index GIN
/// inutilisables sans aucune erreur visible — seulement des seq scans.
fn search_text(alias: &str, columns: &[&str]) -> String {
    let parts: Vec<String> = columns
        .iter()
        .map(|column| format!("coalesce({alias}.{column}, '')"))
        .collect();

    format!("public.immutable_unaccent({})", parts.join(" || ' ' || "))
}

/// Expression `tsvector` indexee (index principal).
pub fn search_vector(alias: &str, columns: &[&str]) -> String {
    format!(
        "to_tsvector('french'::regconfig, {})",
        search_text(alias, columns)
    )
}

/// Expression texte normalisee indexee en trigrammes (index de repli).
pub fn search_trigram(alias: &str, columns: &[&str]) -> String {
    format!("lower({})", search_text(alias, columns))
}

/// Resultat de l'analyse d'une requete texte par `public.search_query`.
#[derive(Debug, QueryableByName)]
pub struct ParsedQuery {
    /// Forme texte de la `tsquery`, a re-caster en `::tsquery` cote requete.
    /// `None` quand la saisie ne contient aucun mot exploitable.
    #[diesel(sql_type = Nullable<Text>)]
    pub tsquery: Option<String>,
    /// Reformulation proposee, non nulle uniquement si au moins un mot a ete
    /// juge fautif. A n'exposer que si la recherche initiale ne ramene rien.
    #[diesel(sql_type = Nullable<Text>)]
    pub suggestion: Option<String>,
}

/// Analyse la saisie utilisateur : construit la `tsquery` augmentee (chaque mot
/// suspect est complete par `| correction`, jamais remplace) et la suggestion
/// affichable.
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

/// Resout un libelle de commune en liste de `code_commune`.
pub async fn resolve_commune(
    connection: &mut Connection,
    commune: &str,
) -> Result<Vec<String>, diesel::result::Error> {
    #[derive(QueryableByName)]
    struct Codes {
        #[diesel(sql_type = diesel::sql_types::Array<Text>)]
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

/// Valeur liee a un placeholder `$n`.
#[derive(Debug, Clone)]
pub enum Bind {
    Text(String),
    Bool(bool),
    Float8(f64),
    Date(NaiveDate),
    TsQuery(String),
    TextArray(Vec<String>),
}

/// Accumulateur de parametres lies.
///
/// Remplace les vecteurs paralleles « nom de champ / index de parametre » de
/// l'ancienne implementation, ou l'ordre des `bind()` devait etre rejoue a
/// l'identique dans la requete principale *et* dans la requete de comptage.
#[derive(Debug, Default)]
pub struct Binder {
    binds: Vec<Bind>,
}

impl Binder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Enregistre une valeur et renvoie le placeholder correspondant.
    pub fn push(&mut self, bind: Bind) -> String {
        self.binds.push(bind);
        format!("${}", self.binds.len())
    }

    pub fn text(&mut self, value: impl Into<String>) -> String {
        self.push(Bind::Text(value.into()))
    }

    /// Applique les valeurs, dans l'ordre d'enregistrement, a une requete brute.
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
            Bind::TextArray(values) => {
                query.bind::<diesel::sql_types::Array<Text>, _>(values.clone())
            }
        })
    }
}
