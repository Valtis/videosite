#!/bin/bash

set -ueo pipefail

if [ "x${1:-}" = "x" ]; then
  echo "Usage: $0 project [test]"
  echo "If 'test' is provided, it will use the test database configuration."
  echo ""
  echo "Project is one of: 'auth'"
  exit 1
fi

if [ "x${2:-}" = "xtest" ]; then
  # In test mode, we use the test database
  export DB_HOST="localhost"
  export DB_PORT="54320"
  export DB_USER="root"
  export DB_PASSWORD="root"
fi


case "$1" in
  auth)
    # Rollback migrations for the auth project
    ./auth/db/rollback_migration.sh
    ;;
  resource)
    # Rollback migrations for the resource project
    ./resource-server/db/rollback_migration.sh
    ;;
  ingestion)
    ./ingestion/db/rollback_migration.sh
    ;;
  audit)
    ./audit/db/rollback_migration.sh
    ;;
  *)
    echo "Unknown project: $1"
    exit 1
    ;;
esac
