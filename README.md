# SIREN API

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

### Usage

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
