# ---- Build stage ----
FROM rust:1.85-slim-bookworm AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# sqlx checks queries against ./.sqlx at compile time instead of a live DB
ENV SQLX_OFFLINE=true

COPY . .
RUN cargo build --release

# ---- Runtime stage ----
FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/mtaalink ./mtaalink
COPY migrations ./migrations

EXPOSE 7878
CMD ["./mtaalink"]
