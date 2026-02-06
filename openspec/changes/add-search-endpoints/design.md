## Context

The API serves French business registry data (SIREN/SIRET) with ~35M establishments and ~25M legal units. The recent `2026-02-06` migration added:
- pg_search BM25 indexes with ngram tokenization on `search_denomination`, `libelle_commune`, `code_postal`, `siren`, `siret`
- PostGIS GIST spatial index on the `position` geography column (generated from Lambert 93 coordinates)
- Generated `search_denomination` columns concatenating denomination fields

Current endpoints only support exact lookup by identifier. No search capability is exposed yet.

## Goals / Non-Goals

**Goals:**
- Expose full-text search, field filtering, and geographic search through two new GET endpoints
- Achieve performant queries by leveraging existing database indexes (BM25, GIST)
- Keep the query builder flexible enough to handle optional combinations of filters
- Maintain consistency with the existing API patterns (response format, error handling, OpenAPI docs)

**Non-Goals:**
- Faceted search / aggregations
- Autocomplete / typeahead (could be a future change)
- Authentication on search endpoints (these are public like existing GET endpoints)
- New database tables or extensions beyond what the `2026-02-06` migration already provides (the migration itself can be adjusted)
- Cursor-based pagination with opaque tokens — simple offset/limit is sufficient for this use case

## Decisions

### 1. Raw SQL via `diesel::sql_query` instead of Diesel query builder

**Choice**: Build SQL strings dynamically and execute via `diesel::sql_query`.

**Why**: The search queries require PostGIS operators (`ST_DWithin`, `<->` for KNN), pg_search operators (`|||` for BM25, `paradedb.score()`), and dynamic WHERE/ORDER BY clauses. Diesel's query builder does not support these custom operators natively, and wrapping them in custom Diesel extensions would be more complex than raw SQL for no real benefit.

**Why not Diesel DSL**: Would require implementing custom `SqlFunction`, `SqlLiteral`, and operator traits for PostGIS KNN (`<->`) and pg_search (`|||`, `paradedb.score`). This is significant boilerplate for operators that are specific to extensions. The queries also need dynamic clause composition (optional filters), which is awkward in typed Diesel DSL.

**Mitigations**: Use parameterized `$N` bind parameters (via `.bind::<Type, _>(value)`) for all user inputs to prevent SQL injection. Keep query building in dedicated functions in the model layer.

### 2. Dynamic query building with typed bind parameters

**Choice**: Build queries by appending SQL fragments conditionally based on which filters are provided, with a parameter counter to track `$N` placeholders.

**Pattern**:
```
let mut conditions: Vec<String>
let mut param_index = 1;
if let Some(code_postal) = &params.code_postal {
    conditions.push(format!("code_postal = ${param_index}"));
    param_index += 1;
}
// ... then bind values in the same order
```

**Why**: This is the simplest approach that keeps queries readable and parameterized. Each filter is optional and independently composable.

**Why not a query builder crate**: Adding a dependency like `sea-query` would be heavy for this use case. The number of filter combinations is bounded and manageable.

### 3. Separate search response types (not reusing existing `Etablissement` struct)

**Choice**: Define lightweight response structs for search results that include only relevant fields plus computed fields (`meter_distance`, `score`).

**Why**: Search results need computed columns (distance, relevance score) that don't exist on the base model. Returning all 50+ columns of `Etablissement` for each search hit would be wasteful. A dedicated struct maps cleanly to `diesel::sql_query` via `#[derive(QueryableByName)]`.

**Trade-off**: More structs to maintain, but each serves a clear purpose and the search response contract is decoupled from the full model.

### 4. Geographic search only on `etablissements`

**Choice**: Geographic filtering (`lat`, `lng`, `radius`) is only available on the `search/etablissements` endpoint.

**Why**: Only `etablissement` has a `position` column. `unite_legale` represents a legal entity, not a physical location. This matches the data model.

### 5. Sorting strategy

**Choice**: Support multiple sort options via a `sort` query parameter with predefined values.

- `etablissements`: `distance` (KNN `<->`), `relevance` (paradedb.score), `date_creation`
- `unites_legales`: `relevance` (paradedb.score), `date_creation`

Default sort: `relevance` when text search is active, `date_creation DESC` otherwise. `distance` requires geographic parameters to be present.

**Why**: Predefined sort values are safer than arbitrary ORDER BY (no SQL injection surface), simpler to validate, and cover the primary use cases.

### 6. Route structure: colocated with existing resource endpoints

**Choice**: Add search as `GET /v3/etablissements?q=...` and `GET /v3/unites_legales?q=...`, alongside the existing `GET /v3/etablissements/{siret}` and `GET /v3/unites_legales/{siren}`.

**Why**: Search and lookup are operations on the same resource. Axum distinguishes `GET /etablissements` (no path param) from `GET /etablissements/{siret}` (with path param) as separate routes, so there is no conflict. This keeps the API surface minimal and RESTful — the collection endpoint returns filtered/searched results, the item endpoint returns a single resource by ID.

## Risks / Trade-offs

**[Offset pagination performance on deep pages]** → For large offsets (e.g., `offset=100000`), PostgreSQL still scans and discards rows. Mitigation: cap `limit` at a reasonable maximum (e.g., 100) and `offset` at a maximum (e.g., 10000). This is acceptable for a search API where deep pagination is rare.

**[Raw SQL maintenance burden]** → Raw SQL is less type-safe than Diesel DSL. Mitigation: keep query-building functions isolated in the model layer with clear parameter documentation. Test with integration tests.

**[KNN operator `<->` with geography filters]** → The `<->` operator on geography uses the GIST index for approximate ordering, which is very fast but may have slight ordering differences vs exact `ST_Distance`. Mitigation: this is standard PostGIS behavior and acceptable for search result ordering. The exact `ST_Distance` is still returned as `meter_distance` for display.

**[pg_search BM25 `|||` operator availability]** → Requires the `pg_search` extension (ParadeDB). If the extension is not installed, queries will fail. Mitigation: the extension is already required by the existing migration. Text search filters should only be applied when a `q` parameter is provided.
