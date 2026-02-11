# AGENTS.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Project Overview

Siren API is a Rust REST API serving French INSEE SIREN/SIRET company data with full-text search (BM25 via ParadeDB pg_search) and geographic search (PostGIS). It downloads bulk data from INSEE, loads it into PostgreSQL, and syncs daily updates via the INSEE API.

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

### Domain Models (`src/models/`)
- **etablissement**: Business establishments (SIRET) - includes geographic search
- **unite_legale**: Legal units (SIREN)
- **lien_succession**: Succession links between entities
- **group_metadata/update_metadata**: Track update state and sync status

Each model follows the pattern: `mod.rs` (queries/CRUD), `common.rs` (structs/types), `error.rs`

### HTTP Routes (`src/commands/serve/runner/`)
Routes map to `/v3/etablissements`, `/v3/unites_legales`, `/v3/etablissements/liens_succession`, and `/admin`.

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

PostgreSQL with extensions:
- `postgis` (geographic queries)
- `pg_search` (BM25 full-text search from ParadeDB)

Schema managed via Diesel migrations in `migrations/`. Run `diesel migration run` after cloning.

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
