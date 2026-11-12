# Build stage
FROM rust:1.88-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY static ./static
COPY config.toml ./config.toml
COPY migrations ./migrations

RUN cargo build --release --locked

# Run stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libsqlite3-0 \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --home /nonexistent --shell /usr/sbin/nologin tinytop \
    && mkdir -p /app/data /app/static \
    && chown -R tinytop:tinytop /app

WORKDIR /app

COPY --from=builder --chown=tinytop:tinytop /app/target/release/tinytop /usr/local/bin/tinytop
COPY --from=builder --chown=tinytop:tinytop /app/static /app/static
COPY --from=builder --chown=tinytop:tinytop /app/config.toml /app/config.toml

ENV TINYTOP_CONFIG=/app/config.toml \
    TINYTOP_DB_PATH=/app/data/tinytop.db \
    TINYTOP_PORT=9090

USER tinytop
EXPOSE 9090
VOLUME ["/app/data"]

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl -fsS http://localhost:9090/api/health || exit 1

ENTRYPOINT ["/usr/local/bin/tinytop"]