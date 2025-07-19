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
    # Drop the database for the auth project
    ./auth/db/drop_database.sh
    ;;
  resource)
    # Drop the database for the resource project
    ./resource-server/db/drop_database.sh
    ;;
  ingestion)
    ./ingestion/db/drop_database.sh
    ;;
  audit)
    ./audit/db/drop_database.sh
    ;;
  *)
    echo "Unknown project: $1"
    exit 1
    ;;
esac
