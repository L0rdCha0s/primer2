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

if [[ -f "$ROOT_DIR/backend/.env" ]]; then
    set -a
    source "$ROOT_DIR/backend/.env"
    set +a
fi

: "${DATABASE_URL:?DATABASE_URL must be set in backend/.env or the environment before running migrations}"

cargo run --manifest-path "$ROOT_DIR/backend/migration/Cargo.toml" -- --database-schema public up

exec cargo run --manifest-path "$ROOT_DIR/backend/Cargo.toml"
