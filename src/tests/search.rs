//! `search-etablissements` and `search-unites-legales` scenarios, played
//! against a real database.

use diesel::QueryableByName;
use diesel::sql_query;
use diesel::sql_types::Text;
use diesel_async::RunQueryDsl;

use crate::models::etablissement::common::{EtablissementSearchParams, EtablissementSortField};
use crate::models::search;
use crate::models::unite_legale::common::{UniteLegaleSearchParams, UniteLegaleSortField};
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

#[tokio::test]
async fn curseur_parcourt_sans_trou_ni_doublon() {
    let mut connection = require_database!();

    // Three cursor pages must reconstitute exactly the same sequence as the same
    // range read in one go.
    let reference = EtablissementSearchParams {
        sort: Some(EtablissementSortField::Siret),
        limit: Some(30),
        ..etablissement_params()
    };
    let reference = etablissement::search(&mut connection, &reference)
        .await
        .unwrap();
    let expected: Vec<String> = reference.results.iter().map(|r| r.siret.clone()).collect();
    assert_eq!(expected.len(), 30);

    let mut collected: Vec<String> = Vec::new();
    let mut cursor: Option<String> = None;
    for _ in 0..3 {
        let page = EtablissementSearchParams {
            sort: Some(EtablissementSortField::Siret),
            limit: Some(10),
            cursor: cursor.clone(),
            ..etablissement_params()
        };
        let page = etablissement::search(&mut connection, &page).await.unwrap();
        collected.extend(page.results.iter().map(|r| r.siret.clone()));
        cursor = page.next_cursor;
        assert!(
            cursor.is_some(),
            "une page pleine doit annoncer la suivante"
        );
    }

    assert_eq!(collected, expected);
}

#[tokio::test]
async fn curseur_respecte_les_filtres() {
    let mut connection = require_database!();

    let first = EtablissementSearchParams {
        commune: Some("paris".to_string()),
        sort: Some(EtablissementSortField::Siret),
        limit: Some(10),
        ..etablissement_params()
    };
    let first = etablissement::search(&mut connection, &first)
        .await
        .unwrap();
    assert_eq!(first.results.len(), 10);

    let second = EtablissementSearchParams {
        commune: Some("paris".to_string()),
        sort: Some(EtablissementSortField::Siret),
        limit: Some(10),
        cursor: first.next_cursor.clone(),
        ..etablissement_params()
    };
    let second = etablissement::search(&mut connection, &second)
        .await
        .unwrap();

    assert!(
        second.results.iter().all(|r| r
            .libelle_commune
            .as_deref()
            .unwrap_or_default()
            .to_lowercase()
            .contains("paris")),
        "le filtre doit continuer de s'appliquer apres reprise"
    );
    let last_of_first = first.results.last().unwrap().siret.clone();
    assert!(
        second.results.iter().all(|r| r.siret > last_of_first),
        "aucune ligne deja rendue ne doit reapparaitre"
    );
}

#[tokio::test]
async fn curseur_va_au_dela_du_plafond_de_decalage() {
    let mut connection = require_database!();

    // 10,000 is the `offset` ceiling, crossed here in pages of 1,000.
    let mut cursor: Option<String> = None;
    let mut seen = 0usize;
    for _ in 0..12 {
        let page = EtablissementSearchParams {
            sort: Some(EtablissementSortField::Siret),
            limit: Some(1_000),
            cursor: cursor.clone(),
            ..etablissement_params()
        };
        let page = etablissement::search(&mut connection, &page).await.unwrap();
        seen += page.results.len();
        cursor = page.next_cursor;
    }

    assert!(
        seen > 10_000,
        "le curseur doit depasser le plafond de offset, vu {seen}"
    );
}

#[tokio::test]
async fn curseur_absent_sur_la_derniere_page() {
    let mut connection = require_database!();

    let params = EtablissementSearchParams {
        q: Some("boulangerie du village".to_string()),
        sort: Some(EtablissementSortField::Siret),
        limit: Some(100),
        ..etablissement_params()
    };
    let output = etablissement::search(&mut connection, &params)
        .await
        .unwrap();

    assert!(output.results.len() < 100, "page incomplete attendue");
    assert!(
        output.next_cursor.is_none(),
        "une page incomplete ne doit pas annoncer de suite"
    );
}

#[tokio::test]
async fn curseur_ignore_sur_les_autres_tris() {
    let mut connection = require_database!();

    // The 400 lives in the HTTP runner; at model level a non-resumable sort must
    // simply not emit a cursor.
    let params = EtablissementSearchParams {
        q: Some("carrefour".to_string()),
        limit: Some(20),
        ..etablissement_params()
    };
    let output = etablissement::search(&mut connection, &params)
        .await
        .unwrap();

    assert!(!output.results.is_empty());
    assert!(output.next_cursor.is_none());
}

#[tokio::test]
async fn curseur_unite_legale() {
    let mut connection = require_database!();

    let first = UniteLegaleSearchParams {
        sort: Some(UniteLegaleSortField::Siren),
        limit: Some(10),
        ..unite_legale_params()
    };
    let first = unite_legale::search(&mut connection, &first).await.unwrap();
    assert_eq!(first.results.len(), 10);
    assert!(first.next_cursor.is_some());

    let second = UniteLegaleSearchParams {
        sort: Some(UniteLegaleSortField::Siren),
        limit: Some(10),
        cursor: first.next_cursor.clone(),
        ..unite_legale_params()
    };
    let second = unite_legale::search(&mut connection, &second)
        .await
        .unwrap();

    let last_of_first = first.results.last().unwrap().siren.clone();
    assert!(second.results.iter().all(|r| r.siren > last_of_first));
}
