# SIREN API

[![Build Status](https://travis-ci.com/Creatiwity/siren.svg?branch=master)](https://travis-ci.com/Creatiwity/siren)
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

### CLI

**> sirene**

```
> sirene -h

Sirene service used to update data in database and serve it through a HTTP REST API

USAGE:
    sirene [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --db-folder <db-folder>        Path to the file storage folder for the database, you can set in environment
                                       variable as DB_FOLDER. Could be the same as FILE_FOLDER if this app and the
                                       database are on the same file system. Files copied by this app inside FILE_FOLDER
                                       must be visible by the database in DB_FOLDER
        --file-folder <file-folder>    Path to the file storage folder for this app, you can set in environment variable
                                       as FILE_FOLDER
        --temp-folder <temp-folder>    Path to the temp folder, you can set in environment variable as TEMP_FOLDER

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
    -k, --api-key <api-key>    API key needed to allow maintenance operation from HTTP, you can set in environment
                               variable as API_KEY
        --env <environment>    Configure log level, you can set in environment variable as SIRENE_ENV [possible values:
                               development, staging, production]
    -h, --host <host>          Listen this host, you can set in environment variable as HOST
    -p, --port <port>          Listen this port, you can set in environment variable as PORT
```

**> sirene update --help**

```
sirene-update
Update data from CSV source files

USAGE:
    sirene update [FLAGS] <group-type> [SUBCOMMAND]

ARGS:
    <group-type>    Configure which part will be updated [possible values: unites-legales, etablissements, all]

FLAGS:
        --data-only    Use an existing CSV file already present in FILE_FOLDER and does not delete it
        --force        Force update even if the source data where not updated
    -h, --help         Prints help information
    -V, --version      Prints version information

SUBCOMMANDS:
    clean-file       Clean files from FILE_FOLDER
    download-file    Download file in TEMP_FOLDER
    finish-error     Set a staled update process to error, use only if the process is really stopped
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
    data_only: bool,
}
```

```
GET /admin/update/status?api_key=string
```

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
