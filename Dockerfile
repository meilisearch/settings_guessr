# ---- Build stage ----
FROM rust:1.95-slim AS builder

WORKDIR /app

# Copy the workspace manifests and lock file
COPY Cargo.toml Cargo.lock ./

# Copy all workspace members
COPY settings_guessr/ ./settings_guessr/
COPY server/ ./server/

# Build only the server binary in release mode
RUN cargo build --release --package server

# ---- Run stage ----
FROM debian:bookworm-slim AS runtime

# Install minimal runtime dependencies (TLS, CA certs, etc.)
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/server /usr/local/bin/server

# The server listens on port 80
EXPOSE 80

ENTRYPOINT ["/usr/local/bin/server"]
