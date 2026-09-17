# Feuille de route — recherche

Suite de la refonte livrée par la migration `2026-09-15-120000_search_fts_commune`
(FTS natif PostgreSQL, correction par lexique, repli trigramme, `commune` en
clair). Ce document liste ce qui reste à faire, par ordre de valeur.

Tous les chiffres qui suivent ont été mesurés sur la base complète
(42,7 M établissements, 29,2 M unités légales), pas estimés.

## Hors périmètre

- **Historisation** (`periode()`, `date=` de l'API Insee). C'est un chantier de
  modèle de données : nos tables ne portent que l'état courant. Écarté
  volontairement.
- **`champs=`** (sélection des champs retournés). La projection de recherche fait
  déjà 15 champs ; le gain de charge utile est marginal et rendrait le schéma
  OpenAPI dynamique.

---

## Lot 1 — Verrouiller l'acquis

### 1.1 Test anti-régression sur l'usage des index — *priorité 1*

L'expression de `src/models/search.rs` doit rester identique **au caractère
près** à celle des index d'expression de la migration. Si quelqu'un ajoute une
colonne au texte recherchable sans toucher la migration (ou l'inverse), rien ne
casse visiblement : les requêtes repassent en *seq scan* et la recherche passe
de 20 ms à 20 s, silencieusement.

Un test qui exécute `EXPLAIN` sur la requête générée et vérifie la présence de
`Bitmap Index Scan on etablissement_search_fts_idx` attrape la divergence
immédiatement.

Effort : petit. C'est le filet de sécurité de tout le reste.

### 1.2 Tests d'intégration sur les scénarios des specs

Il n'y a aucun test aujourd'hui (`cargo test` → 0 test). Les scénarios sont déjà
rédigés dans `openspec/specs/search-etablissements/spec.md` et
`openspec/specs/search-unites-legales/spec.md`. Quelques milliers de lignes de
fixture suffisent pour la validation fonctionnelle (pas pour la performance).

Effort : moyen. À mener idéalement en même temps que le lot 3, pour que les
nouveaux filtres naissent avec leurs tests.

### 1.3 Instrumentation

- Compteur sur le déclenchement du **repli trigramme** (OpenTelemetry et Sentry
  sont déjà branchés).
- Relevé périodique de `pg_stat_user_indexes`.

Objectif concret : décider dans deux ou trois mois, sur données, si l'on
supprime les index trigramme (**857 Mo** à eux deux) et si
`etablissement_filter_idx` (**1 954 Mo**) sert encore. C'est aussi ce relevé qui
tranchera le lot 7.

Effort : petit.

---

## Lot 2 — Index sur `date_creation`

Le meilleur rapport valeur/coût du plan, mesuré :

| requête | avant | après |
| --- | ---: | ---: |
| plage de dates triée (année 2024) | 10 885 ms | **0,68 ms** |
| listing par défaut (tri date, sans filtre) | 3 899 ms | **0,80 ms** |

```sql
CREATE INDEX etablissement_date_creation_idx
  ON etablissement (date_creation DESC NULLS LAST);
```

**287 Mo, 14,6 s de construction.** Corrige une faiblesse préexistante (le
listing par défaut sans filtre) et conditionne l'utilisabilité du lot 3.

À prévoir aussi sur `unite_legale`, et éventuellement sur `date_debut` si les
tris sur ce champ sont utilisés.

---

## Lot 3 — Filtrage riche

C'est la valeur réelle derrière la « syntaxe par champ » de l'API Insee
(`q=denominationUniteLegale:X AND codePostalEtablissement:Y`).

**Ne pas implémenter un langage de requête à la Lucene** : parseur à écrire et à
maintenir, surface d'injection à surveiller, plans d'exécution imprévisibles.
Des paramètres nommés délivrent le même usage pour une fraction du coût et du
risque.

- **Multi-valeurs** : `activite_principale=10.71C,47.24Z` → `= ANY($n)`.
  Mesuré, fonctionne sur les index existants.
- **Plages de dates** : `date_creation_min` / `date_creation_max`, idem pour
  `date_debut`. 29 ms combiné à `q`, 0,68 ms avec l'index du lot 2.
- **Négation** : `activite_principale_not=`, ou un préfixe `!` sur la valeur.

Manque le plus criant à corriger au passage : **`etablissement` n'a aujourd'hui
aucun filtre de date**, et `unite_legale` n'a que l'égalité exacte sur
`date_creation` / `date_debut`, ce qui est inutilisable en pratique.

Le `Binder` introduit dans `src/models/search.rs` rend l'ajout mécanique.

Effort : moyen.

---

## Lot 4 — Facettes

`facette=activite_principale,code_commune` se résout en un `GROUP BY` sur le même
sous-ensemble borné à 10 000 lignes que le calcul de `total` — donc borné par
construction.

Mesuré : **6,5 ms**.

```
q=boulangerie → 10.71C: 7314 | 10.71A: 380 | 15.8C: 335 | 68.20B: 306 | 47.24Z: 222
q=carrefour   → 31555: 144  | 59350: 105  | 06088: 92   | 33063: 83   | 75115: 71
```

C'est ce qui permet de construire une véritable interface de recherche à
facettes.

Effort : petit à moyen.

---

## Lot 5 — `total` plafonné explicite

Un client ne peut pas distinguer « exactement 10 000 résultats » de « au moins
10 000 ». Ajouter `total_capped: bool` à la réponse.

Effort : trois lignes.

---

## Lot 6 — Phonétisation dans le lexique

Couvre le cas que l'API Insee traite (`.phonetisation`) et que nous ne traitons
pas : « erbusse » → « Herbusse ». La distance d'édition ne peut pas l'attraper,
la phonétique si.

`fuzzystrmatch` est déjà installé. Ajouter une colonne `dmetaphone(word)` avec un
btree sur `search_lexicon`, utilisée comme seconde source de candidats quand le
trigramme n'en produit aucun.

La troncature à 4 caractères de `dmetaphone` — rédhibitoire si on l'appliquait
aux 42 M de lignes du corpus — est sans conséquence sur les 241 k mots cibles du
lexique : les collisions produisent simplement des candidats que l'on reclasse
par fréquence. C'est le bon endroit pour cette technique.

À valider avec le même protocole que la correction actuelle : rappel mesuré par
type de faute (suppression, insertion, substitution, transposition) et par
longueur de mot.

Effort : petit.

---

## Lot 7 — Pagination par curseur

Débloque l'export exhaustif et `commune=paris` au-delà de 10 000 résultats
(`offset` est plafonné à 10 000). Keyset sur `(date_creation, siret)`, ou
`(score, siret)` pour le tri par pertinence en ajoutant `siret` comme départage.

Effort : moyen. **Valeur conditionnelle** : si personne ne pagine au-delà de la
dixième page, à repousser. L'instrumentation du lot 1.3 tranchera.

---

## Lot 8 — Exploitation

### 8.1 COPY sans index dans le pipeline mensuel

Les 42 M de lignes sont aujourd'hui chargées dans `*_staging` avec pkey, index
`siren`, index `date_dernier_traitement`, GiST `position` et deux GIN maintenus
en ligne. Les supprimer avant le COPY et les reconstruire après coûte, mesuré :

| index | reconstruction |
| --- | ---: |
| GIN FTS | 25 s |
| GIN trigramme | 21 s |
| btree commune + date | ~3 min |
| btree `date_creation` (lot 2) | 15 s |

Très probablement bien inférieur au surcoût actuel du COPY — à chiffrer sur une
exécution réelle avant de s'engager.

### 8.2 `docker-compose.yml`

Il est sur `postgres:12` sans PostGIS : l'exemple ne démarre pas pour un nouveau
contributeur.

---

## Ordre recommandé

1. **1.1 + 2 + 5** — peu d'effort, effet immédiat, et 1.1 protège tout le reste.
2. **3 + 4** — comblent réellement l'écart fonctionnel avec l'API Insee, et
   partagent la même plomberie de construction de requête.
3. **6** — petit, et ferme un manque nommé.
4. **8** — dès qu'une fenêtre de maintenance est disponible.
5. **7** — seulement si les données d'usage le justifient.
6. **1.2** — en continu, de préférence adossé au lot 3.

---

## Écart résiduel avec l'API Insee, pour mémoire

Ce que l'Insee a et que nous n'aurons toujours pas après ce plan :

- l'historisation (`periode()`, `date=`) — hors périmètre assumé ;
- la syntaxe booléenne libre dans `q` — remplacée par des paramètres nommés ;
- le `total` exact — nous restons sur un comptage borné, désormais signalé.

Ce que nous avons et que l'Insee n'a pas :

- la recherche géographique (`lat`/`lng`/`radius`, tri par distance) ;
- la tolérance aux fautes de frappe sur la dénomination, avec suggestion ;
- `commune` en clair et tolérant aux fautes ;
- le tri par pertinence ;
- aucun quota (l'API Insee est limitée à 30 requêtes/minute) ;
- les liens de succession et la redirection des SIREN doublons.
