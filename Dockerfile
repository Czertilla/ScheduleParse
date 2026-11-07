# syntax=docker/dockerfile:1

# ---- Builder ----------------------------------------------------------
# aws-lc-rs (the TLS crypto backend pulled in by reqwest's rustls feature)
# builds native C code, so the builder stage needs a full toolchain. We only
# ship the resulting binary onward, so this stage's size doesn't matter.
FROM rust:1-slim-bookworm AS builder
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends cmake gcc g++ make perl pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Cache dependency compilation separately from application code.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && cargo build --release \
    && rm -rf src

COPY src ./src
# Touch main.rs so cargo doesn't reuse the dummy build's stale object file.
RUN touch src/main.rs && cargo build --release

# ---- Runtime ------------------------------------------------------------
# Debian slim keeps the aws-lc-rs/rustls TLS stack working out of the box
# (glibc + system CA store) while staying small enough for a 1 vCPU / 1 GB /
# 5 GB disk VM.
FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --no-create-home --uid 10001 appuser

COPY --from=builder /app/target/release/skedule_parse /app/skedule_parse

USER appuser
EXPOSE 8080
ENTRYPOINT ["/app/skedule_parse"]
