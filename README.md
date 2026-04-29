# Primer

Primer is an adaptive, story-driven learning platform built to make tutoring feel personal, alive, and consequential. It takes inspiration from *The Diamond Age* and turns that idea into a working product: a beautifully rendered interactive book that teaches through narrative, remembers the student, adapts to their progress, and unlocks the next challenge only when they are ready.

This is not a static lesson reader. Primer is a responsive learning companion. It combines a skeuomorphic book interface, student memory, AI-guided lesson planning, generated educational artifacts, and stagegate-based progression into one focused experience. The result is a platform that can meet a student where they are, explain concepts in a way that fits their background, and make learning feel like entering a story rather than completing a worksheet.

## Purpose

Primer exists to give every student a more capable, more attentive tutor than traditional software can provide. It is designed for students who need instruction that adapts to them in real time: their interests, biography, strengths, gaps, pace, and demonstrated mastery.

The platform turns learning into a guided journey. Each lesson is delivered as part of an unfolding book, with rich text, memory-aware narration, infographics, and explicit checkpoints. Students do not simply consume content. They respond, demonstrate understanding, and advance through levels as the system verifies readiness.

## What Primer Delivers

- **A real book experience:** The hero surface is a page-turning Primer, built to feel tactile, memorable, and premium. Lessons live inside a book rather than a generic dashboard.
- **Adaptive lessons from student context:** Primer uses the authenticated student profile and biography to generate an opening lesson that feels directly relevant to the learner.
- **Persistent learning memory:** Student progress, memory, and stage state are persisted in Postgres, giving the platform continuity across sessions instead of treating each lesson as isolated.
- **AI-guided topic and lesson planning:** The backend uses OpenAI-powered reasoning to shape topic guidance, adaptive communication, lesson planning, and stagegate grading.
- **Generated visual teaching artifacts:** Primer can generate infographics for lessons using image generation, while still providing graceful fallbacks when live AI credentials are unavailable.
- **Stagegate progression:** Students move forward by proving understanding. The system can evaluate a response, unlock Level 2, and keep the experience grounded in mastery.
- **Production-minded architecture:** A Rust API, typed JSON contracts, SeaORM persistence, migrations, Postgres, pgvector, and Apache AGE give the MVP a serious foundation.

## Value To Students

Primer gives students a learning experience that feels individualized from the first page. A student signs up with a biography, opens the book, and receives a lesson that understands who they are. The experience is personal without becoming chaotic, structured without becoming rigid, and ambitious without losing clarity.

For students, the value is direct:

- They get explanations that connect to their own life and interests.
- They learn through story, which makes abstract material easier to remember.
- They receive visual support through generated infographics and fallback artifacts.
- They progress through explicit mastery checks rather than passive page completion.
- They return to a system that remembers their state and can continue the journey.
- They interact with a product that feels special, polished, and worth paying attention to.

Primer's promise is bold: software should not merely present curriculum. It should tutor. It should adapt. It should challenge. It should remember. It should make the student feel like the lesson was written for them because, in the most important ways, it was.

## Repository Layout

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
