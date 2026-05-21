## 1. Migration base de données

- [x] 1.1 Créer une migration Diesel ajoutant les tables `siren_doublons` et `siren_doublons_staging` (colonnes : `siren_doublon` VARCHAR(9) PK, `siren_canonique` VARCHAR(9) NOT NULL, `date_dernier_traitement_doublon` DATE nullable)
- [x] 1.2 Ajouter dans la même migration (ou une migration dédiée) la ligne `group_metadata` pour `SirenDoublons` avec l'URL `https://object.files.data.gouv.fr/data-pipeline-open/siren/stock/StockDoublons_utf8.zip`
- [x] 1.3 Mettre à jour `src/models/schema.rs` pour refléter les nouvelles tables (auto-généré par `diesel migration run`)

## 2. Modèle `siren_doublon`

- [x] 2.1 Créer `src/models/siren_doublon/mod.rs` et `src/models/siren_doublon/common.rs` avec les structs Diesel (`SirenDoublon`, `SirenDoublonStaging`)
- [x] 2.2 Créer `src/models/siren_doublon/error.rs` avec le type d'erreur du modèle
- [x] 2.3 Implémenter `fn find_canonical_siren(conn: &mut Connection, siren: &str) -> Result<Option<String>, Error>` dans `src/models/siren_doublon/mod.rs`
- [x] 2.4 Implémenter `SirenDoublonModel` avec le trait `UpdatableModel` : `insert_remote_file_in_staging` (truncate staging + COPY CSV), `swap` (truncate production + insert from staging), `count` / `count_staging`, no-op pour `update_daily_data` / `get_total_count`, `None` pour `get_last_insee_synced_timestamp`
- [x] 2.5 Ajouter `pub mod siren_doublon;` dans `src/models/mod.rs`

## 3. Intégration pipeline `GroupType`

- [x] 3.1 Ajouter `SirenDoublons` à l'enum `GroupType` dans `src/models/group_metadata/common.rs` (avec les implémentations `ToSql` / `FromSql` et `Display`)
- [x] 3.2 Ajouter `GroupType::SirenDoublons => Box::new(SirenDoublonModel {})` dans `get_updatable_model()`
- [x] 3.3 Ajouter `SirenDoublons` à `SyntheticGroupType` dans `src/models/update_metadata/common.rs` (ToSql/FromSql/Display + `Vec<GroupType>` pour `All` inclut `SirenDoublons`)
- [x] 3.4 Ajouter `CmdGroupType::SirenDoublons` dans `src/commands/common.rs` et sa conversion vers `SyntheticGroupType`

## 4. Handlers HTTP — redirection 301

- [x] 4.1 Dans `src/commands/serve/runner/error.rs`, ajouter la variante `SirenDoublonRedirect { location: String }` au type `Error` et mapper vers une réponse `301 Moved Permanently` avec header `Location`
- [x] 4.2 Modifier `get_unite_legale_by_siren` dans `src/commands/serve/runner/unites_legales.rs` : après avoir capturé `UniteLegaleNotFound`, appeler `models::siren_doublon::find_canonical_siren` et retourner `Err(Error::SirenDoublonRedirect { location: ... })` si un canonique est trouvé
- [x] 4.3 Modifier `get_etablissement_by_siret` dans `src/commands/serve/runner/etablissements.rs` : après avoir capturé `EtablissementNotFound`, extraire le SIREN (9 premiers chars du SIRET), appeler `find_canonical_siren`, si trouvé récupérer le siège du SIREN canonique et retourner `Err(Error::SirenDoublonRedirect { location: /v3/etablissements/<siret_siege> })`
- [x] 4.4 Mettre à jour les annotations `#[utoipa::path]` des deux handlers pour documenter le code 301

## 5. Vérification

- [x] 5.1 Vérifier que `cargo build` passe sans erreur
- [x] 5.2 Lancer `update siren-doublons` localement et vérifier que la table `siren_doublons` est peuplée
- [x] 5.3 Tester manuellement `GET /v3/unites_legales/<siren_doublon>` → 301 vers le SIREN canonique
- [x] 5.4 Tester manuellement `GET /v3/etablissements/<siret_dont_siren_est_doublon>` → 301 vers le siège canonique
- [x] 5.5 Vérifier que `GET /v3/unites_legales/<siren_inconnu>` retourne toujours 404
