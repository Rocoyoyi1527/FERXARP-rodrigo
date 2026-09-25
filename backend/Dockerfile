# Etapa 1: Compilación con toolchain actualizado
FROM rust:slim AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY .sqlx ./.sqlx
COPY migrations ./migrations

ENV SQLX_OFFLINE=true
RUN cargo build --release

# Etapa 2: Imagen final con la misma versión de GLIBC
FROM debian:trixie-slim

WORKDIR /app

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/backend /app/backend

ENV RUST_LOG=info
EXPOSE 8000

CMD ["/app/backend"]
