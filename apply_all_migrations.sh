#!/bin/bash

set -ueo pipefail

if [ "x${1:-}" = "xtest" ]; then
  # In test mode, we use the test database
  export DB_HOST="localhost"
  export DB_PORT="54320"
  export DB_USER="root"
  export DB_PASSWORD="root"
fi


./auth/db/apply_migrations.sh
./resource-server/db/apply_migrations.sh

