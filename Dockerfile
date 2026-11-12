# Build stage
FROM rust:1.88-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock* ./
COPY src ./src
COPY static ./static
COPY config.toml ./config.toml
COPY migrations ./migrations

RUN cargo build --release

# Run stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/tinytop /usr/local/bin/tinytop
COPY --from=builder /app/static /app/static
COPY --from=builder /app/config.toml /app/config.toml

ENV TINYTOP_CONFIG=/app/config.toml
EXPOSE 9090

ENTRYPOINT ["/usr/local/bin/tinytop"]