# Production Dockerfile for deltav
# Multi-stage build: builder compiles the Rust binary, runtime runs it.

# --- Stage 1: Builder ---
FROM rust:1.94-slim AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy manifests first for layer caching
COPY Cargo.toml Cargo.lock ./
COPY deltav/Cargo.toml deltav/Cargo.toml

# Create a dummy main.rs so cargo can fetch and compile dependencies
RUN mkdir -p deltav/src && echo 'fn main() {}' > deltav/src/main.rs
RUN cargo build --release
RUN rm -rf deltav/src

# Copy actual source and build
COPY deltav/src deltav/src
RUN cargo build --release

# --- Stage 2: Runtime ---
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/deltav /usr/local/bin/deltav

# Create mount point directories
RUN mkdir -p /data /config

EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/deltav", "serve"]
