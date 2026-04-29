#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

export COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME:-primerlab_test}"
export PRIMERLAB_DB_PORT="${PRIMERLAB_DB_PORT:-15434}"
export BIND_ADDR="${BIND_ADDR:-127.0.0.1:4100}"
export DATABASE_URL="${DATABASE_URL:-postgres://primerlab:primerlab@127.0.0.1:${PRIMERLAB_DB_PORT}/primerlab}"

exec "$ROOT_DIR/run.sh"
