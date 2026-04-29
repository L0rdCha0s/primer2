# Primer Build Instructions

We are building Primer, a hackathon MVP for an adaptive story-driven tutor inspired by The Primer in The Diamond Age. The 'hero piece' is the skeumorphic book with turnable pages, and we need to make sure that rendering looks great with correct layout

## Repo Layout

- `frontend/`: Next.js App Router, TypeScript, Tailwind, `react-pageflip`
- `backend/`: Rust API using Poem

## Frontend

- Run frontend commands from `frontend/` or with `npm --prefix frontend`.
- The primary student surface is a real-book style `react-pageflip` view.
- Render lesson text, memory state, stagegates, and infographics inside book pages.
- Infographic generation is a backend tool call using OpenAI image generation (`gpt-image-2`) with graceful fallback if no key is present.
- Do not add frontend demo seed data for learner state; generate the opening lesson from the authenticated student profile through the backend.
- This repo uses a current Next.js version. Before changing framework behavior, read the relevant local docs in `frontend/node_modules/next/dist/docs/`.

## Backend

- Run backend commands from `backend/` or with `cargo --manifest-path backend/Cargo.toml`.
- Keep API routes under `/api/...`.
- Return typed or schema-stable JSON. The demo path must keep working without live AI credentials.
- Do not expose OpenAI API keys to frontend code.
- Load OpenAI credentials from `backend/.env`.
- Start local infrastructure with `docker compose up -d db` or `./run.sh`.
- PostgreSQL must have `vector` and `age` extensions available; Docker builds a Postgres 16 image from `pgvector/pgvector:pg16` and compiles Apache AGE.
- Use SeaORM for student/auth/memory/progress persistence.
- Use the migration crate at `backend/migration` for schema changes.
- Use OpenAI Responses API for topic guidance, adaptive communication, memory-aware lesson planning, and stagegate grading.
- Use the image generation API with `gpt-image-2` for generated infographic artifacts.
- Persist per-student progress/memory in Postgres; do not reintroduce JSON-file persistence for learner state.

## Done When

- `npm --prefix frontend run test` passes.
- `npm --prefix frontend run lint` passes.
- `npm --prefix frontend run build` passes.
- `cargo test --manifest-path backend/Cargo.toml` passes.
- `cargo check --manifest-path backend/Cargo.toml` passes.
- `cargo check --manifest-path backend/migration/Cargo.toml` passes.
- Main demo path works: sign up a student with a biography, load the generated opening book, view or fall back from the infographic tool, submit the stagegate, and see Level 2 unlock.

## Not Negotiable
- Ensure that all modules in the frontend and backend are kept small and tight, and focused. Modularise wherre things grow beyond control
- Ensure the API logs detailed information about all requests to the CLI stdout so we can see what's happening, and so you can debug in test mode
- For every code task, run the relevant backend and frontend test suites before calling the task done. Default to `npm --prefix frontend run test` and `cargo test --manifest-path backend/Cargo.toml`, plus the Done When checks for touched areas; explicitly report any command that could not be run.

## System
- We have a psql client at /Applications/Postgres.app/Contents/Versions/latest/bin/psql that you can use to connect to our docker postgresql instance.
- For testing, to ensure we don't stomp on the running API - spin up a parallel instance of everything required (database, frontend, backend)
