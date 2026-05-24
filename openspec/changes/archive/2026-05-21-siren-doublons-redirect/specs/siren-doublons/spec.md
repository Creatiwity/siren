## ADDED Requirements

### Requirement: Ingestion du fichier StockDoublons dans le pipeline de mise à jour

Le système SHALL ingérer le fichier `StockDoublons_utf8.zip` (colonnes `siren`, `sirenDoublon`, `dateDernierTraitementDoublon`) via le pipeline existant UpdateData + SwapData, de la même manière que les fichiers StockEtablissement et StockUniteLegale.

#### Scenario: Mise à jour du stock doublons (fichier modifié)

- **WHEN** l'action `update-data` est exécutée pour le groupe `siren-doublons` (ou `all`)
- **AND** le fichier distant a une date de modification plus récente que la dernière importation
- **THEN** le système télécharge, décompresse et insère le CSV dans la table de staging `siren_doublons_staging`

#### Scenario: Mise à jour ignorée (fichier non modifié)

- **WHEN** l'action `update-data` est exécutée pour le groupe `siren-doublons`
- **AND** le fichier distant n'a pas été modifié depuis la dernière importation
- **THEN** le système ne réimporte pas le fichier et indique "already imported"

#### Scenario: Swap staging vers production

- **WHEN** l'action `swap-data` est exécutée pour le groupe `siren-doublons`
- **AND** des données ont été insérées en staging
- **THEN** le contenu de `siren_doublons_staging` remplace celui de `siren_doublons` (truncate + insert)

#### Scenario: Groupe `all` inclut les doublons

- **WHEN** une mise à jour est lancée avec le groupe `all`
- **THEN** les doublons SIREN sont mis à jour dans le même workflow que les établissements et unités légales

### Requirement: Lookup d'un SIREN doublon vers son SIREN canonique

Le système SHALL exposer une fonction de lookup permettant de retrouver le SIREN canonique à partir d'un SIREN doublon, utilisée en cas de ressource non trouvée sur les endpoints établissement et unité légale.

#### Scenario: SIREN doublon connu

- **WHEN** `find_canonical_siren` est appelé avec un SIREN présent dans `siren_doublons.siren_doublon`
- **THEN** la fonction retourne `Some(siren_canonique)`

#### Scenario: SIREN non présent dans les doublons

- **WHEN** `find_canonical_siren` est appelé avec un SIREN absent de `siren_doublons`
- **THEN** la fonction retourne `None`
