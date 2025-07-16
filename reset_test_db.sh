#!/bin/bash

set -ueo pipefail

./drop_database.sh auth test
./drop_database.sh resource test
./drop_database.sh ingestion test
./apply_all_migrations.sh test

DB_HOST="127.0.0.1"
DB_PORT="54320"
DB_USER="root"
DB_PASSWORD="root"

USER_DB_CONNECTION_STRING="postgres://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/user_db?sslmode=disable"
INGESTION_DB_CONNECTION_STRING="postgres://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/ingestion_db?sslmode=disable"

psql ${USER_DB_CONNECTION_STRING} < auth/db/user/test_dataset/data.sql
psql ${INGESTION_DB_CONNECTION_STRING} < ingestion/db/ingestion/test_dataset/data.sql