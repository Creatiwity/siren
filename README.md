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
