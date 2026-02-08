## Why

The API currently only supports lookup by exact identifiers (SIRET/SIREN). Users need to discover establishments and legal units through search — by name, location, or a combination of criteria. The database infrastructure for full-text search and spatial queries is already in place and needs to be exposed through API endpoints.

## What Changes

- Add `GET /v3/etablissements` endpoint with:
  - Full-text search on `search_denomination` and `libelle_commune` (using pg_search BM25 `|||` operator with ngram)
  - Field filters: `etat_administratif`, `code_postal`, `siren`, `code_commune`, `activite_principale`, `etablissement_siege`
  - Geographic radius filtering from a reference point (lat/lng + radius in meters, using `ST_DWithin`)
  - Sorting by geographic distance (`<->` KNN operator), text relevance (`paradedb.score`), `date_creation`, `date_debut`
  - Pagination (`limit` + `offset`)
- Add `GET /v3/unites_legales` endpoint with:
  - Full-text search on `search_denomination` (using pg_search BM25 `|||` operator with ngram)
  - Field filters: `etat_administratif`, `date_creation`, `date_debut`, `activite_principale`, `categorie_juridique`, `categorie_entreprise`
  - Sorting by text relevance, `date_creation`, `date_debut`
  - Pagination (`limit` + `offset`)
- Use raw SQL queries (via `diesel::sql_query`) to leverage PostGIS KNN operators and pg_search scoring functions that are not expressible through the Diesel query builder

## Capabilities

### New Capabilities

- `search-etablissements`: Search endpoint for establishments with full-text, geographic, and field filtering
- `search-unites-legales`: Search endpoint for legal units with full-text and field filtering

### Modified Capabilities

- `http-server-axum`: Adding new search routes to the Axum router

## Impact

- **API**: Two new public collection endpoints (`GET /v3/etablissements`, `GET /v3/unites_legales`)
- **Code**: New route handlers, query models, request/response types, raw SQL query builders
- **Dependencies**: No new crate dependencies — `diesel` (with `sql_query`), `postgis_diesel`, and `pg_search` extensions are already available
- **Database**: The `2026-02-06` migration (part of this branch) adds generated columns (`search_denomination`, `position`), BM25 indexes (pg_search), and a GIST spatial index. This migration can be modified if indexes need to be improved.
