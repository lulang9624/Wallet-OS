# Stage 1: Build
FROM rust:1-slim-bookworm as builder

WORKDIR /usr/src/app

# Install dependencies for compilation
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy manifest
COPY Cargo.toml .
# Create dummy main to cache deps
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy source
COPY src ./src
# Touch main to force rebuild
RUN touch src/main.rs
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime libs
RUN apt-get update && apt-get install -y libssl3 ca-certificates sqlite3 && rm -rf /var/lib/apt/lists/*

# Copy binary
COPY --from=builder /usr/src/app/target/release/wallos-next /app/wallos-next
# Copy static files
COPY static /app/static

EXPOSE 80
ENV DATABASE_URL=sqlite:wallos.db

CMD ["./wallos-next"]
