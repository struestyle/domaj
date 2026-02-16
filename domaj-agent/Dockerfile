# Build stage
FROM rust:1-alpine AS builder

RUN apk add --no-cache musl-dev

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
FROM alpine:3.21

RUN apk add --no-cache ca-certificates

WORKDIR /app

# Copy the binary and dependency manifest (for Trivy vulnerability scanning)
COPY --from=builder /app/target/release/domaj-agent /app/domaj-agent
COPY --from=builder /app/Cargo.lock /app/Cargo.lock

EXPOSE 3001

CMD ["/app/domaj-agent"]
