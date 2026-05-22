## Why

Le fichier `StockDoublons_utf8.zip` de l'INSEE répertorie les SIREN devenus des doublons (fusions, absorptions) et leur SIREN canonique de remplacement. Aujourd'hui, toute requête sur un SIREN doublon renvoie un 404 silencieux, alors que l'INSEE fournit la donnée nécessaire pour guider le client vers la bonne ressource via une redirection 301.

## What Changes

- Ajout de l'import du fichier `StockDoublons_utf8.zip` dans le pipeline de mise à jour existant (aux côtés des fichiers StockEtablissement et StockUniteLegale).
- Création d'une table `siren_doublons` en base avec les colonnes `siren_doublon`, `siren_canonique` et `date_dernier_traitement_doublon`.
- Modification du handler `GET /v3/etablissements/<siret>` : si le SIRET n'existe pas, on extrait le SIREN (9 premiers chiffres), on cherche dans `siren_doublons` ; si trouvé, on renvoie une redirection **301** vers l'établissement siège du SIREN canonique.
- Modification du handler `GET /v3/unites_legales/<siren>` : si le SIREN n'existe pas, on cherche dans `siren_doublons` ; si trouvé, on renvoie une redirection **301** vers l'unité légale canonique.
- Comportement 404 inchangé si le SIREN n'est pas non plus présent dans les doublons.

## Capabilities

### New Capabilities

- `siren-doublons`: Ingestion du fichier StockDoublons et lookup SIREN→SIREN canonique, utilisé pour les redirections 301 sur les endpoints établissement et unité légale.

### Modified Capabilities

- `http-server-axum`: Les endpoints `/v3/etablissements/<siret>` et `/v3/unites_legales/<siren>` peuvent désormais renvoyer un **301** (en plus du 200 et du 404 existants) quand le SIREN est un doublon connu.

## Impact

- **Base de données** : nouvelle migration Diesel ajoutant la table `siren_doublons`.
- **Update pipeline** : téléchargement et parsing d'un fichier CSV supplémentaire (`StockDoublons_utf8.zip`) lors des mises à jour.
- **API** : les handlers établissement et unité légale voient leur logique de not-found enrichie d'une vérification de doublon ; le contrat de réponse reste rétrocompatible (nouveau code 301 optionnel, pas de suppression de comportement existant).
- **Dépendances** : aucune dépendance externe nouvelle ; réutilisation de l'infrastructure de téléchargement/décompression/import CSV existante.
