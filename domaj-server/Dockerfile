# Frontend build stage
FROM node:20-alpine AS web-builder

WORKDIR /web
COPY domaj-web/package.json domaj-web/package-lock.json* ./
RUN npm ci || npm install
COPY domaj-web/ ./
RUN npm run build

# Backend build stage
FROM rust:1-alpine AS server-builder

RUN apk add --no-cache musl-dev

WORKDIR /app

# Copy manifests
COPY domaj-server/Cargo.toml ./
COPY domaj-server/Cargo.lock ./

# Create a dummy main.rs to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies only
RUN cargo build --release && rm -rf src

# Copy actual source
COPY domaj-server/src ./src

# Build the application
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM alpine:3.21

RUN apk add --no-cache ca-certificates

WORKDIR /app

# Copy the binary and dependency manifest (for Trivy vulnerability scanning)
COPY --from=server-builder /app/target/release/domaj-server /app/domaj-server
COPY --from=server-builder /app/Cargo.lock /app/Cargo.lock

# Copy frontend static build to public directory
COPY --from=web-builder /web/build /app/public

# Create data directory
RUN mkdir -p /data

EXPOSE 3000

CMD ["/app/domaj-server"]
