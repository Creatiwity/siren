#! /bin/bash

print_usage() {
    echo "deploy - deploy Sirene docker compose"
    echo " "
    echo "deploy LOG_LEVEL SIRENE_ENV POSTGRES_USER POSTGRES_PASSWORD POSTGRES_DB DOCKER_UID DOCKER_GID DOCKER_USER"
    echo " "
    echo "options:"
    echo "LOG_LEVEL                 trace, debug, info, warn, error"
    echo "SIRENE_ENV                development, staging or production"
    echo "POSTGRES_USER             user for postgres"
    echo "POSTGRES_PASSWORD         password for postgres"
    echo "POSTGRES_DB               database name for postgres"
    echo "DOCKER_UID                id user to be used by docker"
    echo "DOCKER_GID                id group to be used by docker"
    echo "DOCKER_USER               user name to be used by docker"
    exit 0;
}

if [ $# -ne 8 ]; then
    echo "Your command line does not contain 8 arguments"
    print_usage
fi

export RUST_LOG="sirene=${1}"
export SIRENE_ENV=$2
export POSTGRES_USER=$3
export POSTGRES_PASSWORD=$4
export POSTGRES_DB=$5
export DOCKER_UID=$6
export DOCKER_GID=$7
export DOCKER_USER=$8
export DATABASE_URL="postgresql://${POSTGRES_USER}:${POSTGRES_PASSWORD}@db:5432/${POSTGRES_DB}"

docker-compose pull
docker-compose down
docker-compose up -d --build
