# Stage 1: Build the application
FROM rust:1.76-slim as builder

# Install dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Create a new empty project
WORKDIR /app
RUN mkdir src

# Copy Cargo.toml and Cargo.lock
COPY Cargo.toml Cargo.lock ./

# Create dummy main.rs to build dependencies
RUN echo "fn main() {}" > src/main.rs
RUN echo "pub fn main() {}" > src/lib.rs

# Build only the dependencies to cache them
RUN cargo build --release

# Remove the dummy files
RUN rm -f src/main.rs src/lib.rs

# Copy the source code
COPY src ./src

# Build the application
RUN touch src/main.rs src/lib.rs
RUN cargo build --release

# Stage 2: Create the runtime image
FROM debian:bullseye-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates libssl-dev && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/zelan-api /usr/local/bin/zelan-api

# Set environment variables
ENV RUST_LOG=info
ENV API_PORT=3000
ENV WEBSOCKET_PORT=8080

# Expose ports
EXPOSE 3000
EXPOSE 8080

# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/zelan-api"]