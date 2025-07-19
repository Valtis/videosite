#!/bin/bash

set -ueo pipefail

# Check that the following environment variables are set: DB_HOST, DB_PORT, DB_USER, DB_PASSWORD

if [[ -z "${DB_HOST:-}" || -z "${DB_PORT:-}" || -z "${DB_USER:-}" || -z "${DB_PASSWORD:-}" ]]; then
  echo "Error: Required environment variables (DB_HOST, DB_PORT, DB_USER, DB_PASSWORD) are not set."
  exit 1
fi

# if DB_USE_SSL is set to true, then use sslmode=require, otherwise use sslmode=disable
if [[ "${DB_USE_SSL:-}" == "true" ]]; then
  SSL_MODE="require"
else
  SSL_MODE="disable"
fi


# Construct the connection string for PostgreSQL
DB_URL="postgres://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/audit_db?sslmode=${SSL_MODE}"

SCRIPT_DIRECTORY="$(dirname "$(readlink -f "$0")")"


migrate -source "file://${SCRIPT_DIRECTORY}/audit" -database "${DB_URL}" down 1