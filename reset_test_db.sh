#!/bin/bash

set -ueo pipefail

./drop_database.sh auth test
./apply_all_migrations.sh test

DB_HOST="127.0.0.1"
DB_PORT="54320"
DB_USER="root"
DB_PASSWORD="root"

CONNECTION_STRING="postgres://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/user_db?sslmode=disable"

psql ${CONNECTION_STRING} < auth/db/user/test_dataset/data.sql
