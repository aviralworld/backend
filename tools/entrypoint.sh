#!/usr/bin/env bash

set -Eeo pipefail

source "$(which docker-entrypoint.sh)"

if [ "$(id -u)" = '0' ]; then
	exec gosu postgres "$BASH_SOURCE" "$@"
fi

if [ -z "$DATABASE_ALREADY_EXISTS" ]; then
	docker_verify_minimum_env
	docker_init_database_dir
	pg_setup_hba_conf

	# only required for '--auth[-local]=md5' on POSTGRES_INITDB_ARGS
	export PGPASSWORD="${PGPASSWORD:-$POSTGRES_PASSWORD}"

	docker_temp_server_start "$@" -c max_locks_per_transaction=256
	docker_setup_db
	docker_process_init_files /docker-entrypoint-initdb.d/*
else
	docker_temp_server_start "$@"
	docker_process_init_files /always-initdb.d/*
fi

export BACKEND_DB_CONNECTION_STRING="postgresql:///$POSTGRES_DB?user=$POSTGRES_USER&port=$POSTGRES_PORT&host=/var/run/postgresql"
cd /usr/app
./initdb

docker_temp_server_stop

exec postgres "$@"
