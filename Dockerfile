FROM rust:1-slim-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release --workspace

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/master /usr/local/bin/master
COPY --from=builder /app/target/release/secondary /usr/local/bin/secondary
