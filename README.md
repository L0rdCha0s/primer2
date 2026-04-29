# PrimerLab

PrimerLab is a hackathon MVP for an adaptive story tutor inspired by The Primer. The app is split into two deployable parts:

- `frontend/`: Next.js App Router, TypeScript, Tailwind, `react-pageflip`
- `backend/`: Rust API scaffold using Poem

## Run Locally

Frontend:

```bash
npm --prefix frontend run dev
```

Backend:

```bash
cargo run --manifest-path backend/Cargo.toml
```

The backend binds to `127.0.0.1:4000` by default. Override it with:

```bash
BIND_ADDR=127.0.0.1:4100 cargo run --manifest-path backend/Cargo.toml
```

## OpenAI Configuration

Put the API key in `backend/.env`:

```bash
OPENAI_API_KEY=sk-...
OPENAI_TEXT_MODEL=gpt-5.5
OPENAI_IMAGE_MODEL=gpt-image-2
```

The backend loads `backend/.env`, calls the Responses API for lesson guidance and stagegate grading, and calls the image generation API with `gpt-image-2` for infographics. Per-student progress is stored locally in `backend/data/students.json` by default.

## Verification

```bash
npm --prefix frontend run lint
npm --prefix frontend run build
cargo check --manifest-path backend/Cargo.toml
```
