## 1. Database migration review

- [x] 1.1 Review the `2026-02-06` migration and verify BM25 indexes cover all filterable columns needed (e.g., `etat_administratif`, `activite_principale` are not currently in the BM25 index — confirm if plain WHERE clauses are sufficient or if index adjustments are needed)

## 2. Search response types and query params

- [x] 2.1 Define `EtablissementSearchParams` struct with query parameter fields: `q`, `etat_administratif`, `code_postal`, `siren`, `code_commune`, `activite_principale`, `etablissement_siege`, `lat`, `lng`, `radius`, `sort`, `limit`, `offset` — with `Deserialize`, `IntoParams` derives for Axum extraction
- [x] 2.2 Define `UniteLegaleSearchParams` struct with query parameter fields: `q`, `etat_administratif`, `activite_principale`, `categorie_juridique`, `categorie_entreprise`, `date_creation`, `date_debut`, `sort`, `limit`, `offset`
- [x] 2.3 Define `EtablissementSearchResult` struct with `QueryableByName` derive containing: `siret`, `siren`, `etat_administratif`, `date_creation`, `search_denomination`, `code_postal`, `libelle_commune`, `activite_principale`, `etablissement_siege`, and optional `meter_distance`, `score`
- [x] 2.4 Define `UniteLegaleSearchResult` struct with `QueryableByName` derive containing: `siren`, `etat_administratif`, `date_creation`, `search_denomination`, `activite_principale`, `categorie_juridique`, `categorie_entreprise`, and optional `score`
- [x] 2.5 Define paginated response wrappers (`EtablissementSearchResponse`, `UniteLegaleSearchResponse`) with `etablissements`/`unites_legales` array, `total`, `limit`, `offset` — with `ToSchema`, `Serialize` derives

## 3. Query builders (model layer)

- [x] 3.1 Implement `search` function in `models/etablissement` that builds a raw SQL query dynamically based on `EtablissementSearchParams`: SELECT columns, optional `ST_Distance` and `paradedb.score`, WHERE clauses for field filters + `ST_DWithin` + `|||` operator, ORDER BY based on sort param, LIMIT/OFFSET — using `$N` bind parameters
- [x] 3.2 Implement `search` function in `models/unite_legale` that builds a raw SQL query dynamically based on `UniteLegaleSearchParams`: SELECT columns, optional `paradedb.score`, WHERE clauses for field filters + `|||` operator, ORDER BY based on sort param, LIMIT/OFFSET — using `$N` bind parameters
- [x] 3.3 Implement parameter validation logic: geographic params must be all-or-none, `sort=distance` requires geo params, `sort=relevance` requires `q`, cap `limit` to 100 (default 20), cap `offset` to 10000 (default 0)
- [x] 3.4 Implement `COUNT(*)` query variant (or use `COUNT(*) OVER()` window function) for `total` in both search functions

## 4. Route handlers

- [x] 4.1 Create `search_etablissements` handler in `src/commands/serve/runner/etablissements.rs` — extract `Query<EtablissementSearchParams>`, validate, call model search function, return `Json<EtablissementSearchResponse>` — with `#[utoipa::path]` OpenAPI annotation
- [x] 4.2 Create `search_unites_legales` handler in `src/commands/serve/runner/unites_legales.rs` — extract `Query<UniteLegaleSearchParams>`, validate, call model search function, return `Json<UniteLegaleSearchResponse>` — with `#[utoipa::path]` OpenAPI annotation
- [x] 4.3 Register the new search handlers in the existing `router()` functions of both modules (add `.routes(routes!(search_etablissements))` and `.routes(routes!(search_unites_legales))`)

## 5. Error handling

- [x] 5.1 Add search-specific error variants to `Error` enum (e.g., `InvalidSearchParams` for validation errors like missing geo params, invalid sort combinations) and map them to HTTP 400

## 6. Verification

- [x] 6.1 Verify the project compiles with `cargo build`
- [x] 6.2 Manual test: search etablissements with text query, field filters, geographic radius, sorting, and pagination
- [x] 6.3 Manual test: search unites legales with text query, field filters, sorting, and pagination
- [x] 6.4 Verify OpenAPI documentation renders correctly in Scalar at `/scalar`
