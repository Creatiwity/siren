//! `search-etablissements` and `search-unites-legales` scenarios, played
//! against a real database.

use diesel::QueryableByName;
use diesel::sql_query;
use diesel::sql_types::Text;
use diesel_async::RunQueryDsl;

use crate::models::etablissement::common::EtablissementSearchParams;
use crate::models::search;
use crate::models::unite_legale::common::UniteLegaleSearchParams;
use crate::models::{etablissement, unite_legale};
use crate::tests::require_database;

fn etablissement_params() -> EtablissementSearchParams {
    EtablissementSearchParams {
        limit: Some(20),
        ..Default::default()
    }
}

fn unite_legale_params() -> UniteLegaleSearchParams {
    UniteLegaleSearchParams {
        limit: Some(20),
        ..Default::default()
    }
}

fn denominations(results: &[etablissement::common::EtablissementSearchResult]) -> Vec<String> {
    results
        .iter()
        .map(|r| {
            [
                r.denomination_usuelle.as_deref(),
                r.enseigne_1.as_deref(),
                r.enseigne_2.as_deref(),
                r.enseigne_3.as_deref(),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
        })
        .collect()
}

// The unit tests in `models::search` check that the Rust expression appears in
// the DDL. These check the other end of the chain: that PostgreSQL actually
// picks the index. Together they close the door on a search that silently falls
// back to a sequential scan.

#[derive(QueryableByName)]
struct Plan {
    #[diesel(sql_type = Text)]
    plan: String,
}

/// `EXPLAIN` returns a column named "QUERY PLAN", which `QueryableByName`
/// cannot bind. A temporary function folds the lines into a nameable column.
async fn query_plan(connection: &mut crate::connectors::local::Connection, sql: &str) -> String {
    sql_query(
        "CREATE OR REPLACE FUNCTION pg_temp.plan_of(query text) RETURNS text \
         LANGUAGE plpgsql AS $$ \
         DECLARE collected text := ''; line text; \
         BEGIN \
           FOR line IN EXECUTE 'EXPLAIN ' || query LOOP \
             collected := collected || line || E'\\n'; \
           END LOOP; \
           RETURN collected; \
         END $$",
    )
    .execute(connection)
    .await
    .expect("creation de pg_temp.plan_of");

    sql_query("SELECT pg_temp.plan_of($1) AS plan")
        .bind::<Text, _>(sql.to_string())
        .get_result::<Plan>(connection)
        .await
        .expect("execution de pg_temp.plan_of")
        .plan
}

#[tokio::test]
async fn index_fts_utilise_sur_etablissement() {
    let mut connection = require_database!();

    let vector = search::search_vector(Some("e"), search::ETABLISSEMENT_SEARCH_COLUMNS);
    let sql = format!(
        "SELECT e.siret FROM etablissement e WHERE {vector} @@ 'carrefour'::tsquery LIMIT 20"
    );

    let plan = query_plan(&mut connection, &sql).await;
    assert!(
        plan.contains("etablissement_search_fts_idx"),
        "le FTS n'utilise pas son index — l'expression a probablement divergé du DDL.\n{plan}"
    );
}

#[tokio::test]
async fn index_fts_utilise_sur_unite_legale() {
    let mut connection = require_database!();

    let vector = search::search_vector(Some("u"), search::UNITE_LEGALE_SEARCH_COLUMNS);
    let sql = format!(
        "SELECT u.siren FROM unite_legale u WHERE {vector} @@ 'carrefour'::tsquery LIMIT 20"
    );

    let plan = query_plan(&mut connection, &sql).await;
    assert!(
        plan.contains("unite_legale_search_fts_idx"),
        "le FTS n'utilise pas son index — l'expression a probablement divergé du DDL.\n{plan}"
    );
}

#[tokio::test]
async fn index_trigramme_utilise_en_repli() {
    let mut connection = require_database!();

    let trigram = search::search_trigram(Some("e"), search::ETABLISSEMENT_SEARCH_COLUMNS);
    let sql = format!(
        "SELECT e.siret FROM etablissement e \
         WHERE lower(public.immutable_unaccent('carrefour')) <% {trigram} LIMIT 20"
    );

    let plan = query_plan(&mut connection, &sql).await;
    assert!(
        plan.contains("etablissement_search_trgm_idx"),
        "le repli trigramme n'utilise pas son index.\n{plan}"
    );
}

#[tokio::test]
async fn recherche_texte_sur_denomination() {
    let mut connection = require_database!();

    let params = EtablissementSearchParams {
        q: Some("carrefour".to_string()),
        ..etablissement_params()
    };
    let output = etablissement::search(&mut connection, &params)
        .await
        .unwrap();

    assert!(!output.results.is_empty());
    assert!(output.results.iter().all(|r| r.score.is_some()));
    assert!(
        denominations(&output.results)
            .iter()
            .any(|d| d.contains("carrefour"))
    );
}

#[tokio::test]
async fn plusieurs_mots_sont_combines_en_et() {
    let mut connection = require_database!();

    let large = EtablissementSearchParams {
        q: Some("boulangerie".to_string()),
        ..etablissement_params()
    };
    let narrow = EtablissementSearchParams {
        q: Some("boulangerie du village".to_string()),
        ..etablissement_params()
    };

    let large = etablissement::search(&mut connection, &large)
        .await
        .unwrap();
    let narrow = etablissement::search(&mut connection, &narrow)
        .await
        .unwrap();

    assert!(
        narrow.total < large.total,
        "ajouter des mots doit restreindre : {} vs {}",
        narrow.total,
        large.total
    );
}

#[tokio::test]
async fn faute_de_frappe_augmente_sans_remplacer() {
    let mut connection = require_database!();

    let params = EtablissementSearchParams {
        q: Some("carefour".to_string()),
        limit: Some(100),
        ..etablissement_params()
    };
    let output = etablissement::search(&mut connection, &params)
        .await
        .unwrap();

    let names = denominations(&output.results);
    assert!(
        names.iter().any(|d| d.contains("carrefour")),
        "la correction doit ramener CARREFOUR"
    );
}

#[tokio::test]
async fn nom_rare_reste_trouvable() {
    let mut connection = require_database!();

    let params = EtablissementSearchParams {
        q: Some("creatiwity".to_string()),
        limit: Some(100),
        ..etablissement_params()
    };
    let output = etablissement::search(&mut connection, &params)
        .await
        .unwrap();

    assert!(
        denominations(&output.results)
            .iter()
            .any(|d| d.contains("creatiwity")),
        "un nom rare ne doit jamais etre ecrase par sa correction"
    );
}

#[tokio::test]
async fn suggestion_uniquement_quand_aucun_resultat() {
    let mut connection = require_database!();

    let trouve = EtablissementSearchParams {
        q: Some("carrefour".to_string()),
        ..etablissement_params()
    };
    let output = etablissement::search(&mut connection, &trouve)
        .await
        .unwrap();
    assert!(
        output.suggestion.is_none(),
        "pas de suggestion quand la recherche aboutit"
    );

    let introuvable = EtablissementSearchParams {
        q: Some("zzzqqqwwwxxx".to_string()),
        ..etablissement_params()
    };
    let output = etablissement::search(&mut connection, &introuvable)
        .await
        .unwrap();
    assert!(output.results.is_empty());
    assert_eq!(output.total, 0);
}

#[tokio::test]
async fn requete_trop_courte_ignoree_par_le_modele() {
    let mut connection = require_database!();

    // The 400 lives in the HTTP runner; at model level a too-short input must
    // simply not filter.
    let params = EtablissementSearchParams {
        q: Some("le".to_string()),
        ..etablissement_params()
    };
    let output = etablissement::search(&mut connection, &params)
        .await
        .unwrap();

    assert!(output.results.iter().all(|r| r.score.is_none()));
}

#[tokio::test]
async fn commune_resolue_par_prefixe() {
    let mut connection = require_database!();

    let params = EtablissementSearchParams {
        commune: Some("paris".to_string()),
        ..etablissement_params()
    };
    let output = etablissement::search(&mut connection, &params)
        .await
        .unwrap();

    assert!(!output.results.is_empty());
    // Matching is on the words of the label, not its start: "paris" also brings
    // back LE TOUQUET-PARIS-PLAGE, just as "etienne" brings back SAINT-ETIENNE.
    assert!(
        output.results.iter().all(|r| r
            .libelle_commune
            .as_deref()
            .unwrap_or_default()
            .to_lowercase()
            .contains("paris")),
        "toutes les lignes doivent etre dans une commune contenant « paris »"
    );
}

#[tokio::test]
async fn commune_tolere_la_faute() {
    let mut connection = require_database!();

    let params = EtablissementSearchParams {
        commune: Some("marseile".to_string()),
        ..etablissement_params()
    };
    let output = etablissement::search(&mut connection, &params)
        .await
        .unwrap();

    assert!(
        output
            .results
            .iter()
            .any(|r| r.libelle_commune.as_deref().unwrap_or_default() == "MARSEILLE"),
        "le repli trigramme sur commune_dim doit rattraper « marseile »"
    );
}

#[tokio::test]
async fn commune_inconnue_renvoie_vide() {
    let mut connection = require_database!();

    let params = EtablissementSearchParams {
        commune: Some("zzzzzzzzzz".to_string()),
        ..etablissement_params()
    };
    let output = etablissement::search(&mut connection, &params)
        .await
        .unwrap();

    assert!(output.results.is_empty());
    assert_eq!(output.total, 0);
    assert!(!output.total_capped);
}

#[tokio::test]
async fn filtre_multi_valeurs() {
    let mut connection = require_database!();

    let params = EtablissementSearchParams {
        code_postal: Some("75001,75002".to_string()),
        limit: Some(100),
        ..etablissement_params()
    };
    let output = etablissement::search(&mut connection, &params)
        .await
        .unwrap();

    assert!(!output.results.is_empty());
    assert!(
        output
            .results
            .iter()
            .all(|r| matches!(r.code_postal.as_deref(), Some("75001") | Some("75002"))),
        "seules les deux valeurs demandees doivent sortir"
    );
}

#[tokio::test]
async fn negation_exclut_la_valeur_et_garde_les_nulls() {
    let mut connection = require_database!();

    let params = EtablissementSearchParams {
        q: Some("boulangerie".to_string()),
        activite_principale_not: Some("10.71C".to_string()),
        limit: Some(100),
        ..etablissement_params()
    };
    let output = etablissement::search(&mut connection, &params)
        .await
        .unwrap();

    assert!(!output.results.is_empty());
    assert!(
        output
            .results
            .iter()
            .all(|r| r.activite_principale.as_deref() != Some("10.71C")),
        "la valeur exclue ne doit plus apparaitre"
    );
}

#[tokio::test]
async fn plage_de_dates() {
    let mut connection = require_database!();

    let min = chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let max = chrono::NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();

    let params = EtablissementSearchParams {
        date_creation_min: Some(min),
        date_creation_max: Some(max),
        limit: Some(100),
        ..etablissement_params()
    };
    let output = etablissement::search(&mut connection, &params)
        .await
        .unwrap();

    assert!(!output.results.is_empty());
    assert!(
        output.results.iter().all(|r| r
            .date_creation
            .map(|d| d >= min && d <= max)
            .unwrap_or(false)),
        "toutes les dates doivent tomber dans les bornes"
    );
}

#[tokio::test]
async fn facettes_calculees_sur_les_champs_demandes() {
    let mut connection = require_database!();

    let params = EtablissementSearchParams {
        q: Some("boulangerie".to_string()),
        facette: Some("activite_principale,code_commune".to_string()),
        ..etablissement_params()
    };
    let output = etablissement::search(&mut connection, &params)
        .await
        .unwrap();

    assert_eq!(output.facettes.len(), 2);

    let activites = output.facettes.get("activite_principale").unwrap();
    assert!(!activites.is_empty());
    assert!(
        activites.len() as i64 <= search::FACET_VALUES_LIMIT,
        "le nombre de valeurs par facette est borne"
    );
    assert!(
        activites.windows(2).all(|w| w[0].nombre >= w[1].nombre),
        "les valeurs doivent etre triees par effectif decroissant"
    );
}

#[tokio::test]
async fn facettes_absentes_si_non_demandees() {
    let mut connection = require_database!();

    let params = EtablissementSearchParams {
        q: Some("boulangerie".to_string()),
        ..etablissement_params()
    };
    let output = etablissement::search(&mut connection, &params)
        .await
        .unwrap();

    assert!(output.facettes.is_empty());
}

#[tokio::test]
async fn total_plafonne_signale() {
    let mut connection = require_database!();

    let large = EtablissementSearchParams {
        q: Some("sarl".to_string()),
        ..etablissement_params()
    };
    let output = etablissement::search(&mut connection, &large)
        .await
        .unwrap();
    assert_eq!(output.total, search::SEARCH_TOTAL_CAP);
    assert!(
        output.total_capped,
        "un total au plafond doit etre signale comme tel"
    );

    let narrow = EtablissementSearchParams {
        q: Some("boulangerie du village".to_string()),
        ..etablissement_params()
    };
    let output = etablissement::search(&mut connection, &narrow)
        .await
        .unwrap();
    assert!(output.total < search::SEARCH_TOTAL_CAP);
    assert!(!output.total_capped);
}

#[tokio::test]
async fn unite_legale_recherche_texte_et_facettes() {
    let mut connection = require_database!();

    let params = UniteLegaleSearchParams {
        q: Some("carrefour".to_string()),
        facette: Some("categorie_juridique".to_string()),
        ..unite_legale_params()
    };
    let output = unite_legale::search(&mut connection, &params)
        .await
        .unwrap();

    assert!(!output.results.is_empty());
    assert!(output.results.iter().all(|r| r.score.is_some()));
    assert_eq!(output.facettes.len(), 1);
    assert!(
        !output
            .facettes
            .get("categorie_juridique")
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn unite_legale_faute_de_frappe() {
    let mut connection = require_database!();

    let params = UniteLegaleSearchParams {
        q: Some("carefour".to_string()),
        limit: Some(100),
        ..unite_legale_params()
    };
    let output = unite_legale::search(&mut connection, &params)
        .await
        .unwrap();

    assert!(
        output.results.iter().any(|r| r
            .denomination
            .as_deref()
            .unwrap_or_default()
            .to_lowercase()
            .contains("carrefour")),
        "la correction doit ramener CARREFOUR"
    );
}
