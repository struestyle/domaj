# Build stage
FROM rust:1-bookworm AS builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml ./

# Create a dummy main.rs to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies only
RUN cargo build --release && rm -rf src

# Copy actual source
COPY src ./src

# Build the application
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary
COPY --from=builder /app/target/release/domaj-agent /app/domaj-agent

EXPOSE 3001

CMD ["/app/domaj-agent"]
