# syntax=docker/dockerfile:1.7

FROM rust:1-bookworm AS builder

WORKDIR /app
COPY backend/Cargo.toml backend/Cargo.lock ./backend/
COPY backend/src ./backend/src

RUN cargo build --manifest-path backend/Cargo.toml --release

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/backend/target/release/primerlab-api /usr/local/bin/primerlab-api

ENV BIND_ADDR=0.0.0.0:4000
EXPOSE 4000

USER 65532:65532
CMD ["primerlab-api"]
