# SIREN API

[![Build Status](https://github.com/Creatiwity/siren/workflows/Build/badge.svg?branch=develop)](https://github.com/Creatiwity/siren/actions?query=workflow%3ABuild)
[![docker pulls](https://img.shields.io/docker/pulls/creatiwity/siren.svg)](https://hub.docker.com/r/creatiwity/siren/)
[![docker image info](https://images.microbadger.com/badges/image/creatiwity/siren.svg)](http://microbadger.com/images/creatiwity/siren)
[![docker tag](https://images.microbadger.com/badges/version/creatiwity/siren.svg)](https://hub.docker.com/r/creatiwity/siren/tags/)

REST API for serving INSEE files v3.

## Getting started

To have a working copy of this project, follow the instructions.

### Installation

Setup [Rust](https://www.rust-lang.org).

Define your environment variables as defined in `.env.sample`. You can either manually define these environment variables or use a `.env` file.

Setup a postgresql database (macOS commands).

```
brew install postgresql
createuser --pwprompt sirene # set password to sirenepw for instance
createdb --owner=sirene sirene
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

1. Go to https://portail-api.insee.fr/catalogue/
2. Create an account or sign in
3. Create an application on this portal
4. Subscribe this application to the _Sirene - V3_ API
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

```
GET /v3/unites_legales/<siren>
GET /v3/etablissements/<siret>
```

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

## Tests

```
cargo test
```

## Deployment

A docker image is built and a sample `docker-compose.yml` with its `docker` folder are usable to test it.

## Authors

-   **Julien Blatecky** - [Julien1619](https://twitter.com/Julien1619)

## License

[MIT](LICENSE.md)
