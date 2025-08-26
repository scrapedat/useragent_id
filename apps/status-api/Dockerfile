# -------- Build Stage --------
FROM rust:1.80-slim AS builder
WORKDIR /app

# Cache deps
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY ../../Cargo.toml ./workspace-Cargo.toml
COPY ../../Cargo.lock ./workspace-Cargo.lock
# minimal synthetic workspace to cache deps
RUN mkdir -p crates/status-api && \
    printf "[workspace]\nmembers=[\n    \"crates/status-api\"\n]\n" > Cargo.toml && \
    printf "[package]\nname=\"status-api\"\nversion=\"0.0.0\"\nedition=\"2021\"\n\n[dependencies]\nanyhow=\"1\"\naxum=\"0.6\"\nchrono=\"0.4\"\nserde=\"1\"\nserde_json=\"1\"\ntokio={version=\"1\",features=[\"full\"]}\ntracing=\"0.1\"\ntracing-subscriber=\"0.3\"\nuuid=\"1\"\n" > crates/status-api/Cargo.toml && \
    mkdir -p crates/status-api/src && \
    printf "fn main(){}" > crates/status-api/src/main.rs && \
    cargo fetch

# Build real app
COPY . /app
RUN cargo build --release

# -------- Runtime Stage --------
FROM gcr.io/distroless/cc-debian12
WORKDIR /app
COPY --from=builder /app/target/release/status-api /app/status-api
ENV PORT=8080
EXPOSE 8080
USER 65532:65532
CMD ["/app/status-api"]
