#!/bin/bash

set -ueo pipefail

migration_name="$1"

migrate create -ext sql -dir audit -seq "$migration_name"