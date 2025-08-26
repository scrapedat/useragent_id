##############################
# Build Stage
##############################
FROM rust:1.80-slim AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Cache dependencies (workspace-aware)
COPY Cargo.toml Cargo.lock ./
COPY apps/status-api/Cargo.toml apps/status-api/Cargo.toml
RUN mkdir -p apps/status-api/src && echo "fn main(){}" > apps/status-api/src/main.rs && \
    cargo build -p status-api --release || true

# Build real binary
COPY . .
RUN cargo build -p status-api --release

##############################
# Runtime Stage
##############################
FROM gcr.io/distroless/cc-debian12
WORKDIR /app
COPY --from=builder /app/target/release/status-api /app/status-api
ENV PORT=8080
EXPOSE 8080
USER 65532:65532
CMD ["/app/status-api"]
