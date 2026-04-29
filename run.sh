#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

cd "$ROOT_DIR"

docker compose up -d db

for _ in {1..30}; do
    if docker compose exec -T db pg_isready -U primerlab -d primerlab >/dev/null 2>&1; then
        break
    fi
    sleep 1
done

DATABASE_URL_WAS_SET=0
DATABASE_URL_OVERRIDE=""
BIND_ADDR_WAS_SET=0
BIND_ADDR_OVERRIDE=""

if [[ ${DATABASE_URL+x} ]]; then
    DATABASE_URL_WAS_SET=1
    DATABASE_URL_OVERRIDE="$DATABASE_URL"
fi

if [[ ${BIND_ADDR+x} ]]; then
    BIND_ADDR_WAS_SET=1
    BIND_ADDR_OVERRIDE="$BIND_ADDR"
fi

if [[ -f "$ROOT_DIR/backend/.env" ]]; then
    set -a
    source "$ROOT_DIR/backend/.env"
    set +a
fi

if [[ "$DATABASE_URL_WAS_SET" -eq 1 ]]; then
    export DATABASE_URL="$DATABASE_URL_OVERRIDE"
fi

if [[ "$BIND_ADDR_WAS_SET" -eq 1 ]]; then
    export BIND_ADDR="$BIND_ADDR_OVERRIDE"
fi

: "${DATABASE_URL:?DATABASE_URL must be set in backend/.env or the environment before running migrations}"

cargo run --manifest-path "$ROOT_DIR/backend/migration/Cargo.toml" -- --database-schema public up

exec cargo run --manifest-path "$ROOT_DIR/backend/Cargo.toml"
