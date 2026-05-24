## Context

L'API expose deux endpoints principaux : `GET /v3/etablissements/<siret>` et `GET /v3/unites_legales/<siren>`. Quand une entité n'existe pas en base, l'API retourne 404. L'INSEE publie un fichier `StockDoublons_utf8.zip` qui recense les SIREN devenus des doublons (absorptions, fusions) avec leur SIREN canonique. Ce fichier suit le même format ZIP/CSV que les autres fichiers stock (StockEtablissement, StockUniteLegale). Le pipeline d'ingestion existant est orchestré via `GroupType` → `UpdatableModel` → actions UpdateData / SwapData / SyncInsee.

## Goals / Non-Goals

**Goals:**
- Ingérer `StockDoublons_utf8.zip` dans le pipeline existant (UpdateData + SwapData uniquement)
- Retourner 301 vers la ressource canonique quand un SIREN doublon est demandé (établissement siège ou unité légale)
- Retourner 404 inchangé quand le SIREN n'est pas non plus dans les doublons

**Non-Goals:**
- Synchronisation quotidienne via l'API INSEE pour les doublons (pas d'API delta disponible)
- Modifier le comportement des endpoints de recherche (search)
- Suivre les doublons en chaîne (siren doublon d'un doublon)

## Decisions

### 1. Réutiliser le pipeline `GroupType` / `UpdatableModel`

Ajouter `GroupType::SirenDoublons` et `SyntheticGroupType::SirenDoublons` en suivant exactement le pattern `LiensSuccession`. `SirenDoublonModel` implémente `UpdatableModel` avec `update_daily_data` et `get_total_count` en no-op (retour immédiat) et `get_last_insee_synced_timestamp` retournant `None` (ce qui fait passer `SyncInseeAction` silencieusement).

**Alternative rejetée** : table gérée hors pipeline, rechargée manuellement. Rejeté car ça crée un chemin de mise à jour séparé difficile à monitorer et qui ne bénéficie pas du mécanisme staging/swap.

### 2. Pattern staging + swap pour la table `siren_doublons`

Suivre le pattern existant : table `siren_doublons_staging` pour l'ingestion, swap vers `siren_doublons` (truncate + insert from staging). Cela garantit que le swap est atomique et que les vérifications de cohérence (count ±1%) s'appliquent.

**Alternative rejetée** : truncate + reload direct sans staging. Rejeté car expose une fenêtre pendant laquelle la table est vide, et perd les garanties du swap.

### 3. `SyntheticGroupType::All` inclut `SirenDoublons`

Ajouter `GroupType::SirenDoublons` dans le `Vec` retourné par `All`. Ajouter aussi `CmdGroupType::SirenDoublons` pour permettre une mise à jour ciblée.

### 4. Lookup doublon dans le handler, pas dans la couche model

La logique "si NotFound → chercher doublon → redirect 301" est une responsabilité de routage. Elle reste dans `runner/etablissements.rs` et `runner/unites_legales.rs`, après avoir capturé l'erreur NotFound. La couche model expose uniquement `models::siren_doublon::find_canonical_siren(conn, siren) -> Result<Option<String>, Error>`.

**Alternative rejetée** : intégrer la recherche de doublon dans `models::etablissement::get` / `models::unite_legale::get`. Rejeté car mélange la recherche de données et la logique de redirection, et complexifie les modèles.

### 5. Réponse 301 avec header `Location`

Utiliser `(StatusCode::MOVED_PERMANENTLY, [(header::LOCATION, url)]).into_response()` dans Axum. Les variantes `Redirect::permanent` d'Axum produisent un 308, pas un 301 — construire la réponse manuellement est nécessaire.

- Pour unité légale : `Location: /v3/unites_legales/{siren_canonique}`
- Pour établissement : `Location: /v3/etablissements/{siret_siege_canonique}` (SIRET du siège de l'unité légale canonique)

### 6. Migration Diesel pour la table et le seed `group_metadata`

Une migration crée les tables `siren_doublons` et `siren_doublons_staging` avec les colonnes `siren_doublon` (PK VARCHAR 9), `siren_canonique` (VARCHAR 9), `date_dernier_traitement_doublon` (DATE nullable). Une deuxième migration (ou la même) insère la ligne dans `group_metadata` avec l'URL du fichier stock.

## Risks / Trade-offs

- **Redirect vers un siège inexistant** → Si le SIREN canonique existe dans les doublons mais n'a pas encore d'établissement siège en base (données incohérentes), le redirect 301 pointe vers une URL qui retournera elle-même 404. Mitigation : la table doublons est mise à jour en même temps que les tables établissements/unités légales dans le même workflow `All`, minimisant la fenêtre d'incohérence.
- **Cascade doublon→doublon** → Un SIREN canonique pourrait lui-même être un doublon. Hors scope pour l'instant ; l'INSEE garantit en général un seul niveau d'indirection.
- **Impact sur le count de `All`** → L'ajout de SirenDoublons dans le workflow `All` allonge légèrement le temps de mise à jour. Mitigation : le fichier doublons est petit (~quelques milliers de lignes).

## Migration Plan

1. Déployer la migration Diesel (nouvelles tables + seed `group_metadata`).
2. Déployer le binaire avec la nouvelle logique.
3. Lancer `update all` (ou `update siren-doublons`) pour ingérer le fichier doublons.
4. Les redirections 301 sont actives dès que la table est peuplée.

**Rollback** : revenir au binaire précédent restaure le 404 direct. La table `siren_doublons` reste mais est ignorée. Une migration de rollback supprime la table et la ligne `group_metadata`.

## Open Questions

- Faut-il exposer `siren_canonique` dans le corps de la réponse 404 pour les clients qui ne suivent pas les redirections ? (hors scope actuel)
- L'URL `StockDoublons_utf8.zip` doit-elle être configurée via variable d'environnement ou seed en base comme les autres ? → Seed en base, cohérent avec le pattern existant.
