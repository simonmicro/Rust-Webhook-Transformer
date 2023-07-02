FROM debian:stable as builder

# Setup rust with cargo
RUN apt-get update && apt-get install -y curl build-essential pkg-config libssl-dev
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Copy the source code
COPY src /app/src
COPY Cargo.lock /app/Cargo.lock
COPY Cargo.toml /app/Cargo.toml
WORKDIR /app

# Build the binary
RUN cargo build --release

# Path: Dockerfile
FROM debian:stable-slim

# Install openssl
RUN apt-get update && apt-get install -y openssl ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/rust-webhook-transformer /usr/local/bin/rust-webhook-transformer

# Add extra metadata to the image
EXPOSE 8080

# Run the binary
CMD ["rust-webhook-transformer"]