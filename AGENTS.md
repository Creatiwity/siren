# AGENTS.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Project Overview

Siren API is a Rust REST API serving French INSEE SIREN/SIRET company data with full-text search (native PostgreSQL `tsvector`, with lexicon-based typo correction and a trigram fallback) and geographic search (PostGIS). It downloads bulk data from INSEE, loads it into PostgreSQL, and syncs daily updates via the INSEE API.

## Development Commands

```bash
# Build
cargo build

# Run server
cargo run -- serve --env development --port 8080 --host localhost

# Run tests
cargo test

# Lint (matches CI)
cargo clippy --all-features

# Auto-reload during development
cargo watch -x 'run -- serve --env development --port 8080'

# Database migrations
diesel migration run
diesel migration generate <migration_name>

# Update data from INSEE
cargo run -- update all
cargo run -- update unites-legales
cargo run -- update etablissements
```

## Architecture

### CLI Commands (`src/commands/`)
- **serve**: HTTP API server via Axum with OpenAPI/Scalar docs at `/scalar`
- **update**: Data sync workflow (download CSV → load staging → swap tables → sync daily from INSEE API)
- **migrate**: Applies the embedded Diesel migrations and exits (`--check` only lists the pending ones, exit 1 if any)

`serve` and `update` migrate on startup unless `--skip-migrations` / `SKIP_MIGRATIONS=true` is set; they then only verify that nothing is pending and panic otherwise. That check reads `__diesel_schema_migrations` directly (`connectors::local::pending_migrations`), because `MigrationHarness` issues a `CREATE TABLE IF NOT EXISTS` that a read replica rejects. The Helm chart runs `migrate` as a pre-install/pre-upgrade hook Job and sets `SKIP_MIGRATIONS=true` on the Deployment and CronJob (`migrations.enabled`). `apiDatabase.*` gives the Deployment alone its own connection (read replica), falling back field by field to `pg*`; the Job and the CronJob always use the primary.

### Geocoding (`src/geocoding/`, feature `geocoding`)
Address geocoding with the `geocoder-core` crate (git dependency, `Creatiwity/geocoder-core`), which reproduces the BAN API ranking. Everything is behind `#[cfg(feature = "geocoding")]`; `cargo clippy --all-features` covers it, plain `cargo check` covers the build without it.
- `sync`: `update geocoding` (and after `update all` unless `GEOCODING_WITH_UPDATE_ALL=false`) downloads the BAN export next to `GEOCODING_INDEX_PATH`, rebuilds the index atomically, and records the source `Last-Modified`/`ETag` in `<index>.source.json` to skip unchanged exports.
- `handle`: `GeocoderHandle`, held by the serve `Context`, reloads the index when the file's modification time changes (`GEOCODING_RELOAD_INTERVAL_SECONDS`).
- `address`: `Adresse` (API shape), the `geocoding_mode` selection (`threshold_or_best`, `threshold`, `best`) and `filter_for` (housenumber 100 m, street/locality 1 km, municipality → `code_commune`, with Paris/Lyon/Marseille arrondissements).
Routes: `/v3/adresses/autocomplete`, `/v3/adresses/search`, and the `address` parameter of `/v3/etablissements` (response field `adresse`). 503 while no index is loaded, 501 without `GEOCODING_INDEX_PATH`. Helm: `geocoding.*` (PVC shared by the API and the jobs, suspended `-geocoding` CronJob for the first build). The default is ReadWriteOnce with `persistence.sameNode`: a required pod affinity puts the API pods and the jobs on one node, since block storage (OVH csi-cinder) cannot be attached to several; use a ReadWriteMany class and `sameNode: false` to spread API replicas.

### Domain Models (`src/models/`)
- **etablissement**: Business establishments (SIRET) - includes geographic search
- **unite_legale**: Legal units (SIREN)
- **lien_succession**: Succession links between entities
- **group_metadata/update_metadata**: Track update state and sync status

Each model follows the pattern: `mod.rs` (queries/CRUD), `common.rs` (structs/types), `error.rs`

### HTTP Routes (`src/commands/serve/runner/`)
Routes map to `/v3/etablissements`, `/v3/unites_legales`, `/v3/etablissements/liens_succession`, and `/admin`.

Probes live in `health.rs`: `/health/live` (process only, never a dependency) and `/health/ready` (database `SELECT 1` under a 2 s timeout, 503 once SIGTERM is received). They are merged after the Sentry/trace layers so probes do not create transactions. `/` is data-freshness metadata, not a probe. On SIGTERM the server fails readiness for `SHUTDOWN_DELAY_SECONDS`, then shuts down gracefully.

Search endpoints use raw SQL with parameterized queries for complex filtering (geographic radius, text search, field filters).

### Connectors (`src/connectors/`)
- **local**: PostgreSQL connection pool via Diesel/r2d2
- **insee**: INSEE API client for daily data sync (requires `INSEE_CREDENTIALS`)

### Update Workflow (`src/update/`)
Three-step workflow: `UpdateData` → `SwapData` → `SyncInsee`
- Downloads zipped CSV from data.gouv.fr
- Loads into `_staging` tables via Diesel COPY
- Atomic table swap to production
- Daily incremental sync from INSEE API

## Database Requirements

PostgreSQL 14+ with extensions (all contrib, available on every managed provider):
- `postgis` (geographic queries)
- `unaccent` (accent folding)
- `pg_trgm` (commune name matching, correction candidates, search fallback)
- `fuzzystrmatch` (Levenshtein re-ranking of correction candidates)

Full-text search itself needs no extension: it uses the core `tsvector`/`tsquery` machinery with the `french` configuration.

Schema managed via Diesel migrations in `migrations/`. Run `diesel migration run` after cloning.

### Search internals

- Text index: GIN expression index on `to_tsvector('french', immutable_unaccent(<denomination + enseignes>))`. The expression lives in `src/models/search.rs` and **must** stay byte-identical to the migration, otherwise the index is silently bypassed.
- `libelle_commune` is deliberately NOT part of the text index. Communes are addressed by the `commune` parameter, resolved through `commune_dim`.
- `search_lexicon` holds the corpus vocabulary with document frequencies; `public.search_query(q, source)` turns a raw query into an augmented `tsquery` (`word | correction`, never a replacement) plus a display suggestion.
- `public.search_refresh_full(source)` rebuilds the lexicon, the commune dimension and the statistics; it runs after each stock swap. `public.search_refresh_incremental(source, since)` merges only the rows touched by the daily Insee sync. Neither blocks reads.
- Typo correction has two candidate sources, consulted in order: trigram similarity re-ranked by Levenshtein, then — only if that yields nothing — the phonetic key `public.phonetic_fr` (metaphone after stripping the silent leading `h`).
- List filters accept comma-separated values and have a `_not` twin; dates are bounded with `_min` / `_max`. Facets (`facette=`) are computed over the same capped subset as `total`, from a whitelist — never from raw user input.
- Bulk loads into `*_staging` drop the non-constraint indexes first and rebuild them from the catalogue definitions afterwards, inside a transaction (see `models::common::load_staging_without_indexes`). Measured 2.6x faster on 1M rows.

### Tests

`cargo test` runs the unit tests alone. Set `SIRENE_TEST_DATABASE_URL` (deliberately distinct from `DATABASE_URL`) to also run the integration suite against a real database.

Two guards protect the search from silently degrading into a sequential scan: a unit test asserts the expression built in `src/models/search.rs` appears verbatim in the index DDL, and an integration test reads the query plan to confirm PostgreSQL picks the index.

## Environment Variables

See `.env.sample`. Key variables:
- `DATABASE_URL`: PostgreSQL connection string
- `SIRENE_ENV`: development/staging/production
- `INSEE_CREDENTIALS`: API key for daily sync (from portail-api.insee.fr)
- `API_KEY`: Required for `/admin` endpoints
- `SENTRY_DSN`: Optional error tracking

## Testing

Tests require a running PostgreSQL instance with extensions. No special test harness—use `cargo test`.

## Deployment

Docker image built via GitHub Actions (`.github/workflows/rust.yml`). Helm chart in `app/` for Kubernetes.
