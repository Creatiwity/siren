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
INSEE_CREDENTIALS=[Base64(consumer-key:consumer-secret)]
```

**How to generate INSEE_CREDENTIALS**

This variable is only needed if you want to have the daily updates.

1. Go to https://api.insee.fr/catalogue/
2. Create an account or sign in
3. Create an application on this portal
4. Subscribe this application to the _Sirene - V3_ API
5. Generate a key pair in the application details
6. Copy the key from the `curl` example and paste it in `.env`: `Authorization: Basic [INSEE_CREDENTIALS]`

### CLI

**> sirene --help**

```
sirene 2.0.0
Julien Blatecky
Sirene service used to update data in database and serve it through a HTTP REST API

USAGE:
    sirene [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --db-folder <db-folder>        Path to the file storage folder for the database, you can set
                                       in environment variable as DB_FOLDER. Could be the same as
                                       FILE_FOLDER if this app and the database are on the same file
                                       system. Files copied by this app inside FILE_FOLDER must be
                                       visible by the database in DB_FOLDER
        --file-folder <file-folder>    Path to the file storage folder for this app, you can set in
                                       environment variable as FILE_FOLDER
        --temp-folder <temp-folder>    Path to the temp folder, you can set in environment variable
                                       as TEMP_FOLDER

SUBCOMMANDS:
    help      Prints this message or the help of the given subcommand(s)
    serve     Serve data from database to /unites_legales/<siren> and /etablissements/<siret>
    update    Update data from CSV source files
```

**> sirene serve --help**

```
sirene-serve
Serve data from database to /unites_legales/<siren> and /etablissements/<siret>

USAGE:
    sirene serve [OPTIONS]

FLAGS:
        --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -k, --api-key <api-key>      API key needed to allow maintenance operation from HTTP, you can
                                 set in environment variable as API_KEY
    -b, --base-url <base-url>    Base URL needed to configure asynchronous polling for updates, you
                                 can set in environment variable as BASE_URL
        --env <environment>      Configure log level, you can set in environment variable as
                                 SIRENE_ENV [possible values: development, staging, production]
    -h, --host <host>            Listen this host, you can set in environment variable as HOST
    -p, --port <port>            Listen this port, you can set in environment variable as PORT
```

**> sirene update --help**

```
sirene-update
Update data from CSV source files

USAGE:
    sirene update [FLAGS] <group-type> [SUBCOMMAND]

ARGS:
    <group-type>    Configure which part will be updated [possible values: unites-legales,
                    etablissements, all]

FLAGS:
        --data-only    Use an existing CSV file already present in FILE_FOLDER and does not delete
                       it
        --force        Force update even if the source data where not updated
    -h, --help         Prints help information
    -V, --version      Prints version information

SUBCOMMANDS:
    clean-file       Clean files from FILE_FOLDER
    download-file    Download file in TEMP_FOLDER
    finish-error     Set a staled update process to error, use only if the process is really
                     stopped
    help             Prints this message or the help of the given subcommand(s)
    insert-data      Load CSV file in database in loader-table from DB_FOLDER
    swap-data        Swap loader-table to production
    sync-insee       Synchronise daily data from INSEE since the last modification
    unzip-file       Unzip file from TEMP_FOLDER, and move it to the FILE_FOLDER
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
