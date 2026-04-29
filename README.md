# PrimerLab

PrimerLab is a hackathon MVP for an adaptive story tutor inspired by The Primer.

- `frontend/`: Next.js App Router, TypeScript, Tailwind, `react-pageflip`
- `backend/`: Rust API using Poem, SeaORM, and Postgres
- `docker/postgres/`: Postgres 16 image with pgvector and Apache AGE

## Local Infrastructure

Start Postgres:

```bash
docker compose up -d db
```

The database is exposed at `127.0.0.1:5432` with:

```text
DATABASE_URL=postgres://primerlab:primerlab@127.0.0.1:5432/primerlab
```

The Docker image is based on `pgvector/pgvector:pg16`, builds Apache AGE into the image, and initializes:

```sql
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS age;
SELECT create_graph('primer_memory');
```

## Backend

Copy or edit `backend/.env`:

```bash
OPENAI_API_KEY=
OPENAI_TEXT_MODEL=gpt-5.5
OPENAI_IMAGE_MODEL=gpt-image-2
BIND_ADDR=127.0.0.1:4000
DATABASE_URL=postgres://primerlab:primerlab@127.0.0.1:5432/primerlab
```

Run migrations:

```bash
cargo run --manifest-path backend/migration/Cargo.toml -- up
```

Run the API:

```bash
cargo run --manifest-path backend/Cargo.toml
```

Or start Postgres, wait for it, migrate, and run the API:

```bash
./run.sh
```

Run an isolated test API without touching the default API or database ports:

```bash
./run-test.sh
```

The test API binds to `127.0.0.1:4100` and its Postgres instance is exposed on
`127.0.0.1:15434` by default. Override `BIND_ADDR` or `PRIMERLAB_DB_PORT` when
you need another pair of ports.

## Frontend

```bash
npm --prefix frontend run dev
```

The frontend expects the API at `http://127.0.0.1:4000` unless `NEXT_PUBLIC_API_BASE_URL` is set.

## Verification

```bash
npm --prefix frontend run lint
npm --prefix frontend run build
cargo check --manifest-path backend/Cargo.toml
cargo check --manifest-path backend/migration/Cargo.toml
```
