# PrimerLab Build Instructions

We are building PrimerLab, a hackathon MVP for an adaptive story-driven tutor inspired by The Primer.

## Repo Layout

- `frontend/`: Next.js App Router, TypeScript, Tailwind, `react-pageflip`
- `backend/`: Rust API using Poem

## Frontend

- Run frontend commands from `frontend/` or with `npm --prefix frontend`.
- The primary student surface is a real-book style `react-pageflip` view.
- Render lesson text, memory state, stagegates, and infographics inside book pages.
- Use deterministic React/HTML/SVG for infographic text instead of generated images.
- Keep demo seed data in `frontend/src/lib/demo-data.ts`.
- This repo uses a current Next.js version. Before changing framework behavior, read the relevant local docs in `frontend/node_modules/next/dist/docs/`.

## Backend

- Run backend commands from `backend/` or with `cargo --manifest-path backend/Cargo.toml`.
- Keep API routes under `/api/...`.
- Return typed or schema-stable JSON. The demo path must keep working without live AI credentials.
- Do not expose OpenAI API keys to frontend code.
- Load OpenAI credentials from `backend/.env`.
- Use OpenAI Responses API for topic guidance, adaptive communication, memory-aware lesson planning, and stagegate grading.
- Use the image generation API with `gpt-image-2` for generated infographic artifacts.
- Persist per-student progress/memory in the backend. The local MVP store is `backend/data/students.json`.

## Done When

- `npm --prefix frontend run lint` passes.
- `npm --prefix frontend run build` passes.
- `cargo check --manifest-path backend/Cargo.toml` passes.
- Main demo path works: load Mina, page through the book, view the lightning infographic, submit the stagegate, and see Level 2 unlock.

## Not Negotiable
- Ensure that all modules in the frontend and backend are kept small and tight, and focused. Modularise wherre things grow beyond control

## System
- We have a psql client at /Applications/Postgres.app/Contents/Versions/latest/bin/psql that you can use to connect to our docker postgresql instance.