# syntax=docker/dockerfile:1.7

FROM node:24.21.0-bookworm-slim AS frontend-check
ENV PNPM_HOME=/pnpm
ENV PATH=$PNPM_HOME:$PATH
WORKDIR /src/frontend
RUN corepack enable && corepack prepare pnpm@12.4.1 --activate
COPY frontend/package.json frontend/pnpm-lock.yaml ./
RUN --mount=type=cache,id=pnpm-store,target=/pnpm/store \
    pnpm config set store-dir /pnpm/store && pnpm install --frozen-lockfile
COPY frontend/ ./
RUN pnpm check && pnpm test --run && pnpm build

FROM rust:1.98.1-trixie AS rust-base
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libudev-dev \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates/ ./crates/
COPY xtask/ ./xtask/
COPY tests/fixtures/ ./tests/fixtures/
COPY --from=frontend-check /src/frontend/build ./frontend/build

FROM rust-base AS rust-check
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=cargo-target,target=/src/target \
    cargo fmt --all -- --check && \
    cargo clippy --workspace --all-targets -- -D warnings && \
    cargo test --workspace

FROM rust-check AS rust-builder
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=cargo-target,target=/src/target \
    cargo build --release --locked -p aooscope-server && \
    mkdir -p /out && cp /src/target/release/aooscope-server /out/aooscope

FROM debian:trixie-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libudev1 tini tzdata \
    && rm -rf /var/lib/apt/lists/*
RUN mkdir -p /app/cfg
COPY --from=rust-builder /out/aooscope /usr/local/bin/aooscope
WORKDIR /app
ENV AOOSCOPE_CONFIG_DIR_IN_CONTAINER=/app/cfg
ENV AOOSCOPE_BIND=0.0.0.0:8765
EXPOSE 8765
ENTRYPOINT ["/usr/bin/tini", "-g", "--"]
CMD ["/usr/local/bin/aooscope"]
