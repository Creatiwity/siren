# SIREN API

[![Build Status](https://github.com/Creatiwity/siren/workflows/Build/badge.svg?branch=develop)](https://github.com/Creatiwity/siren/actions?query=workflow%3ABuild)
[![docker pulls](https://img.shields.io/docker/pulls/creatiwity/siren.svg)](https://hub.docker.com/r/creatiwity/siren/)
[![docker image info](https://images.microbadger.com/badges/image/creatiwity/siren.svg)](http://microbadger.com/images/creatiwity/siren)
[![docker tag](https://images.microbadger.com/badges/version/creatiwity/siren.svg)](https://hub.docker.com/r/creatiwity/siren/tags/)

REST API for serving INSEE files v3 with full-text search and geographic search capabilities.

## Getting started

To have a working copy of this project, follow the instructions.

### Installation

1. **Setup Rust**: Install [Rust](https://www.rust-lang.org) (version 1.70+ recommended)

2. **Environment variables**: Define your environment variables as defined in `.env.sample`. You can either manually define these environment variables or use a `.env` file.

3. **PostgreSQL database**: Setup PostgreSQL with required extensions (macOS commands):

```bash
brew install postgresql
createuser --pwprompt sirene # set password to sirenepw for instance
createdb --owner=sirene sirene

# Connect to database and enable required extensions
psql -U sirene -d sirene
CREATE EXTENSION IF NOT EXISTS postgis;
CREATE EXTENSION IF NOT EXISTS pg_search;
\q
```

4. **Required PostgreSQL extensions**:
   - `postgis` (for geographic search)
   - `pg_search` (for full-text search with BM25 from ParadeDB)

5. **Optional**: For development, you may want to install:
```bash
brew install diesel_cli  # For database migrations
cargo install cargo-watch  # For auto-reloading during development
```

## Documentation

### Configuration

Recommended configuration for production with docker:

```
RUST_LOG=sirene=warn
SIRENE_ENV=production
BASE_URL=[Your base URL, needed to update asynchronously]
API_KEY=[Any randomized string, needed to use the HTTP admin endpoint]
DATABASE_URL=postgresql://[USER]:[PASSWORD]@[PG_HOST]:[PG_PORT]/[PG_DATABASE]
DATABASE_POOL_SIZE=100
INSEE_CREDENTIALS=[API_KEY]
```

**How to generate INSEE_CREDENTIALS**

This variable is only needed if you want to have the daily updates.

1. Go to https://portail-api.insee.fr/catalog/all
2. Create an account or sign in
3. Create an application on this portal
4. Subscribe this application to API SIRENE (Sirene 4 - v3.11)
5. Generate a key in the application details
6. Copy the key paste it in `.env` instead of `[API_KEY]`

### CLI

**> sirene --help**

```
Sirene service used to update data in database and serve it through a HTTP REST API

Usage: sirene <COMMAND>

Commands:
  update  Update data from CSV source files
  serve   Serve data from database to /unites_legales/<siren> and /etablissements/<siret>
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

**> sirene serve --help**

```
Serve data from database to /unites_legales/<siren> and /etablissements/<siret>

Usage: sirene serve [OPTIONS] --env <ENVIRONMENT> --port <PORT> --host <HOST>

Options:
      --env <ENVIRONMENT>    Configure log level [env: SIRENE_ENV=development] [possible values: development, staging, production]
      --port <PORT>          Listen this port [env: PORT=3000]
      --host <HOST>          Listen this host [env: HOST=localhost]
      --api-key <API_KEY>    API key needed to allow maintenance operation from HTTP [env: API_KEY=]
      --base-url <BASE_URL>  Base URL needed to configure asynchronous polling for updates [env: BASE_URL=http://localhost:3000]
  -h, --help                 Print help
```

**> sirene update --help**

```
Update data from CSV source files

Usage: sirene update [OPTIONS] <GROUP_TYPE> [COMMAND]

Commands:
  update-data   Download, unzip and load CSV file in database in loader-table
  swap-data     Swap loader-table to production
  sync-insee    Synchronise daily data from INSEE since the last modification
  finish-error  Set a staled update process to error, use only if the process is really stopped
  help          Print this message or the help of the given subcommand(s)

Arguments:
  <GROUP_TYPE>  Configure which part will be updated [possible values: unites-legales, etablissements, all]

Options:
      --force  Force update even if the source data where not updated
  -h, --help   Print help
```

### HTTP API

#### Lookup Endpoints

```
GET /v3/unites_legales/<siren>
GET /v3/etablissements/<siret>
```

#### Search Endpoints (NEW!)

**Search Establishments**
```
GET /v3/etablissements?q=<text>&lat=<latitude>&lng=<longitude>&radius=<meters>&sort=<field>&direction=<asc|desc>&limit=<number>&offset=<number>
```

**Search Legal Units**
```
GET /v3/unites_legales?q=<text>&sort=<field>&direction=<asc|desc>&limit=<number>&offset=<number>
```

**Query Parameters**:

- `q`: Full-text search query (searches in denomination and commune name for establishments, denomination only for legal units)
- `lat`, `lng`, `radius`: Geographic search (establishments only) - filters results within radius meters from (lat,lng) point
- `sort`: Sort field - `distance` (geo only), `relevance` (text search), `date_creation`, `date_debut`
- `direction`: Sort direction - `asc` or `desc` (defaults to sensible values per sort field)
- `limit`: Results per page (default: 20, max: 100)
- `offset`: Pagination offset (default: 0, max: 10000)
- `etat_administratif`: Filter by administrative status (A=active, F=closed)
- `code_postal`: Filter by postal code
- `siren`: Filter by SIREN (establishments only)
- `code_commune`: Filter by commune code
- `activite_principale`: Filter by main activity code
- `etablissement_siege`: Filter by headquarters status (establishments only)
- `categorie_juridique`: Filter by legal category (legal units only)
- `categorie_entreprise`: Filter by company category (legal units only)
- `date_creation`: Filter by creation date (legal units only)
- `date_debut`: Filter by start date (legal units only)

**Maintenance**

_This API is enabled only if you have provided an API_KEY when starting the `serve` process._

```
POST /admin/update

{
    api_key: string,
    group_type: "UnitesLegales" | "Etablissements" | "All",
    force: bool,
    asynchronous: bool,
}
```

If `asynchronous` is set to `true`, the update endpoint will immediately return the following:

```
Status: 202 Accepted
Location: /admin/update/status?api_key=string
Retry-After: 10

[Initial status for the started update]
```

```
GET /admin/update/status?api_key=string
```

If an update is in progress, the status code will be 202, otherwise 200.

```
POST /admin/update/status/error

{
    api_key: string,
}
```

### Basic usage

Serve:

```
cargo run serve
```

Update:

```
cargo run update all
```

Help:

```
cargo run help
```

## Features

### Core Features
- REST API for INSEE SIREN/SIRET data
- Automatic updates from INSEE API
- PostgreSQL backend with efficient indexing
- Docker support for easy deployment

### New Search Features (v5.0+)
- **Full-text search**: BM25 algorithm with n-gram tokenization for partial matches
- **Geographic search**: Radius filtering and distance-based sorting using PostGIS
- **Field filtering**: Filter by administrative status, activity codes, dates, etc.
- **Flexible sorting**: By relevance, distance, or dates
- **Pagination**: Efficient offset/limit pagination with accurate total counts

### Technical Features
- **PostgreSQL extensions**: PostGIS for spatial data, pg_search for full-text search
- **Optimized queries**: Raw SQL with parameterized queries for performance
- **OpenAPI documentation**: Complete API documentation via Scalar
- **Async support**: Optional asynchronous updates for large datasets

## Tests

```bash
cargo test
```

## Deployment

A docker image is built and a sample `docker-compose.yml` with its `docker` folder are usable to test it.

### Docker Setup

```bash
docker-compose up -d
```

### Environment Variables

Required for production:
```
RUST_LOG=sirene=warn
SIRENE_ENV=production
BASE_URL=https://your-domain.com
API_KEY=your-secret-key
DATABASE_URL=postgresql://user:password@db:5432/sirene
DATABASE_POOL_SIZE=100
INSEE_CREDENTIALS=your-insee-api-key
```

## Development

### Running locally

```bash
# Start the server
cargo run -- serve --env development --port 8080 --host 0.0.0.0

# Run tests
cargo test

# Run with auto-reload
cargo watch -x 'run -- serve --env development --port 8080'
```

### Database Migrations

```bash
# Run migrations
diesel migration run

# Create new migration
diesel migration generate migration_name
```

## API Documentation

The API includes comprehensive OpenAPI documentation accessible at:
- `/scalar` - Interactive Scalar API documentation
- `/openapi.json` - OpenAPI specification

## Examples

### Search Establishments

```bash
# Text search
curl "http://localhost:8080/v3/etablissements?q=boulangerie&limit=5"

# Geographic search (within 1km of Eiffel Tower)
curl "http://localhost:8080/v3/etablissements?lat=48.8584&lng=2.2945&radius=1000&sort=distance"

# Combined search with filters
curl "http://localhost:8080/v3/etablissements?q=restaurant&code_postal=75001&etat_administratif=A&sort=relevance&limit=10"
```

### Search Legal Units

```bash
# Text search with sorting
curl "http://localhost:8080/v3/unites_legales?q=creati&sort=date_creation&direction=desc&limit=5"

# Filter by activity code
curl "http://localhost:8080/v3/unites_legales?activite_principale=62.01Z&categorie_juridique=5710"
```

## Authors

- **Julien Blatecky** - [@Julien1619](https://twitter.com/Julien1619)

## License

[MIT](LICENSE.md)
