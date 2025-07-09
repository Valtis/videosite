#!/bin/bash

set -ueo pipefail

migration_name="$1"

migrate create -ext sql -dir user -seq "$migration_name"