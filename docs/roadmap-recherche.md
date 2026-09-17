# Feuille de route — recherche

Suite de la refonte livrée par la migration `2026-09-15-120000_search_fts_commune`
(FTS natif PostgreSQL, correction par lexique, repli trigramme, `commune` en
clair).

Tous les chiffres ont été mesurés sur la base complète (42,7 M établissements,
29,2 M unités légales), pas estimés.

## État

| Lot | Sujet | État |
| --- | --- | --- |
| 1.1 | Garde-fous sur l'usage des index | livré |
| 1.2 | Tests d'intégration sur les scénarios des specs | livré — 29 tests |
| 1.3 | Instrumentation du repli trigramme | livré |
| 2 | Index sur `date_creation` | livré |
| 3 | Filtrage riche (multi-valeurs, plages, négation) | livré |
| 4 | Facettes | livré |
| 5 | `total` plafonné explicite | livré |
| 6 | Phonétisation dans le lexique | livré, avec une limite connue |
| 7 | Pagination par curseur | **reporté**, conditionné aux données d'usage |
| 8.1 | Chargement de staging sans index | livré |
| 8.2 | `docker-compose.yml` | livré |

## Hors périmètre

- **Historisation** (`periode()`, `date=` de l'API Insee). Chantier de modèle de
  données : nos tables ne portent que l'état courant. Écarté volontairement.
- **`champs=`** (sélection des champs retournés). La projection de recherche fait
  déjà 15 champs ; le gain de charge utile est marginal et rendrait le schéma
  OpenAPI dynamique.

---

## Lot 1 — Garde-fous

### 1.1 Dérive entre l'expression indexée et l'expression interrogée

C'est le mode de panne le plus coûteux de cette architecture, et le moins
visible : si l'expression de `src/models/search.rs` diverge de celle du DDL
(une colonne ajoutée d'un côté seulement), rien ne casse — les requêtes
repassent simplement en *seq scan* et la recherche passe de 20 ms à 20 s.

Deux tests ferment la porte, un à chaque bout de la chaîne :

- `models::search::tests::expressions_fts_identiques_au_ddl` compare
  l'expression construite en Rust au texte de la migration, aux espaces près.
  Aucune base requise, s'exécute en quelques microsecondes.
- `tests::search::index_fts_utilise_sur_etablissement` lit le plan d'exécution
  réel et vérifie que PostgreSQL choisit bien l'index.

Le premier a été validé par sabotage volontaire : ajouter `libelle_commune` aux
colonnes indexées fait échouer les deux tests d'expression avec un message qui
nomme précisément la divergence.

### 1.2 Tests d'intégration

28 tests, dont 23 adossés à une vraie base, couvrant les scénarios des specs :
recherche texte, ET implicite entre les mots, correction sans écrasement,
préservation des noms rares, suggestion seulement sur résultat vide, résolution
de commune (préfixe, faute de frappe, inconnue), filtres multi-valeurs,
négation, plages de dates, facettes, plafonnement du total.

```bash
cargo test                                   # unitaires seuls
SIRENE_TEST_DATABASE_URL=… cargo test        # + intégration
```

`SIRENE_TEST_DATABASE_URL` est volontairement distinct de `DATABASE_URL` : un
`cargo test` ne doit pas pouvoir toucher la base de développement par accident.
Sans la variable, les tests d'intégration se mettent en sommeil en l'annonçant,
plutôt que de passer en silence.

### 1.3 Instrumentation

Le déclenchement du repli trigramme émet un évènement `tracing` sur la cible
`sirene::search`, avec la source et la longueur de la requête.

Objectif : décider dans deux ou trois mois, sur données, si l'on supprime les
index trigramme (**857 Mo** à eux deux) et si `etablissement_filter_idx`
(**1 954 Mo**) sert encore. Ce même relevé tranchera le lot 7.

---

## Lot 2 — Index sur `date_creation`

| requête | avant | après |
| --- | ---: | ---: |
| plage de dates triée (année 2024) | 10 885 ms | **0,68 ms** |
| listing par défaut (tri date, sans filtre) | 3 899 ms | **0,80 ms** |

**287 Mo, 14,6 s de construction.** Corrige une faiblesse préexistante — le
listing par défaut sans filtre — et conditionne l'utilisabilité du lot 3. Posé
aussi sur `unite_legale`.

Pas d'index sur `date_debut` : à ajouter seulement si les tris sur ce champ
s'avèrent utilisés, pour ne pas payer 287 Mo de plus sans raison.

---

## Lot 3 — Filtrage riche

La valeur réelle derrière la « syntaxe par champ » de l'API Insee, sans en payer
le prix : **pas de langage de requête à la Lucene** — parseur à maintenir,
surface d'injection, plans imprévisibles. Des paramètres nommés délivrent le
même usage pour une fraction du coût.

- **Multi-valeurs** : `code_postal=75001,75002`, `activite_principale=10.71C,47.24Z`
- **Exclusion** : `activite_principale_not=`, `code_postal_not=`, `code_commune_not=`
  (et les trois équivalents sur `unite_legale`). Les lignes dont le champ est nul
  sont conservées : exclure une valeur ne dit rien des valeurs inconnues.
- **Plages** : `date_creation_min` / `_max`, `date_debut_min` / `_max`, bornes
  incluses et chacune facultative.

Manque comblé au passage : `etablissement` n'avait **aucun** filtre de date, et
`unite_legale` n'avait que l'égalité exacte, inutilisable en pratique. Les deux
paramètres exacts restent acceptés pour compatibilité ascendante.

---

## Lot 4 — Facettes

`facette=activite_principale,code_commune` se résout en un `GROUP BY` sur le même
sous-ensemble borné à 10 000 lignes que le calcul de `total` — donc borné par
construction, et en un seul aller-retour quel que soit le nombre de champs.

Mesuré : **6,5 ms** en SQL, 76 ms bout en bout via l'API sur `q=boulangerie`
avec deux facettes.

```
q=boulangerie → activite_principale: 10.71C=7314, 10.71A=380, 15.8C=335
                code_commune:        06088=93, 67482=49, 44109=48
```

Les champs autorisés sont une liste blanche ; tout autre champ donne un 400 qui
énumère les valeurs acceptées, plutôt qu'un silence. Un test vérifie qu'une
tentative d'injection ne franchit pas ce filtre.

---

## Lot 5 — `total` plafonné explicite

`total_capped: bool` distingue « exactement 10 000 » de « au moins 10 000 ».

---

## Lot 6 — Phonétisation dans le lexique

Couvre les fautes d'oreille, que la distance d'édition ne rattrape pas.

`public.phonetic_fr` = `metaphone(…, 8)` précédé du retrait du `h` initial, muet
en français. Le choix mérite d'être justifié, car j'ai d'abord essayé
`dmetaphone` comme le suggérait le plan initial — mesuré, il est moins bon :

| mot | `dmetaphone` | `phonetic_fr` |
| --- | --- | --- |
| `philippe` / `filipe` | FLP / FLP | FLP / FLP |
| `boulangerie` / `boulengerie` | PLNK / PLNK | BLNJR / BLNJR |
| `hotel` / `otel` | HTL / **ATL** | OTL / OTL |
| `herbusse` / `erbusse` | HRPS / **ARPS** | ERBS / ERBS |

Le code sur 8 caractères réduit aussi les collisions sur un lexique de 137 k
mots cibles. Colonne générée `search_lexicon.phonetic` + index btree partiel de
**1,5 Mo**. La source phonétique n'est consultée que si le trigramme n'a produit
aucun candidat, pour ne pas dégrader la qualité existante.

Résultat mesuré, là où la correction précédente échouait :

```
filipe      →  philippe      (avant : rien)
fotographe  →  photographie
```

**Limite connue** : le cas `erbusse` → `Herbusse` de la documentation Insee ne
fonctionne pas chez nous, non pas à cause de l'algorithme — `phonetic_fr` les
regroupe bien — mais parce que `herbusse` n'est pas dans le lexique : c'est un
patronyme rare, sous le seuil de fréquence `ndoc >= 5` qui définit les cibles de
correction valides. L'Insee phonétise le corpus entier ; nous phonétisons un
vocabulaire filtré par fréquence, précisément pour éviter que les fautes
présentes dans les données ne se légitiment elles-mêmes. C'est un arbitrage
assumé, pas un oubli.

---

## Lot 7 — Pagination par curseur — *reporté*

Débloquerait l'export exhaustif et `commune=paris` au-delà de 10 000 résultats
(`offset` est plafonné à 10 000). Keyset sur `(date_creation, siret)`, ou
`(score, siret)` pour le tri par pertinence en ajoutant `siret` comme départage.

Volontairement non implémenté : le plan le conditionnait aux données d'usage, et
elles n'existent pas encore. L'instrumentation du lot 1.3 tranchera.

---

## Lot 8 — Exploitation

### 8.1 Chargement de staging sans index

Mesure sur 1 M de lignes d'établissements, 9 index dont deux GIN et un GiST :

| stratégie | durée |
| --- | ---: |
| chargement avec tous les index (avant) | 36,9 s |
| sans aucun index, puis reconstruction totale | 17,9 s |
| **clé primaire conservée, 8 index retirés** | **14,2 s** |

C'est la troisième qui est livrée : maintenir la clé primaire pendant le
chargement coûte peu, alors que la reconstruire ensuite coûtait 4,9 s à elle
seule. **×2,6** sur le chargement mensuel.

Les définitions d'index sont relues dans le catalogue plutôt qu'écrites en dur :
ce que la migration a créé est exactement ce qui est restauré, sans risque de
dérive — la même discipline que le lot 1.1. Le tout est dans une transaction :
un échec en cours de chargement ramène les index, sans quoi un plantage au
mauvais moment laisserait une table de staging sans index, que le swap
promouvrait en production. Deux tests couvrent ce chemin, dont celui du retour
arrière.

### 8.2 `docker-compose.yml`

Passé de `postgres:12` (sans PostGIS, donc inutilisable) à `postgis/postgis:17-3.5`,
avec un *healthcheck* et une dépendance `service_healthy`. La clé `version:`,
obsolète en Compose v2, a été retirée.

---

## Latences après implémentation

API réelle, caches chauds, base complète.

| requête | latence |
| --- | ---: |
| listing par défaut (tri date) | 6 ms |
| `q=boulangerie du village` | 8 ms |
| `commune=paris` | 25 ms |
| `q=filipe` (correction phonétique) | 40 ms |
| `q=carefour` (correction de frappe) | 50 ms |
| `q=carrefour` | 56 ms |
| `q=boulangerie&facette=activite_principale,code_commune` | 76 ms |
| `q=sarl` | 111 ms |
| géo 1 km, tri distance | 158 ms |
| plage de dates 2024 | 232 ms |
