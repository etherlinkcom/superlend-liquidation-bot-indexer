# Builder stage
FROM rust:1.84.1-alpine3.21 as builder

# Install build dependencies
RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    openssl-libs-static \
    pkgconfig \
    git \
    make \
    gcc \
    libc-dev \
    ca-certificates

# Set OpenSSL configuration for static linking
ENV OPENSSL_STATIC=yes
ENV OPENSSL_DIR=/usr

# Create a new empty shell project
WORKDIR /app

# Add aarch64-musl target
RUN rustup target add aarch64-unknown-linux-musl

# Copy source code
COPY Cargo.toml ./
COPY indexer indexer/
COPY indexer_database indexer_database/

# Build statically linked release for ARM64
ENV RUSTFLAGS='-C target-feature=+crt-static'
RUN cargo build --release --bin indexer --target aarch64-unknown-linux-musl

# Runtime stage - using Alpine instead of scratch for SSL certificates
FROM alpine:3.21

# Install SSL certificates
RUN apk add --no-cache ca-certificates

WORKDIR /app

# Copy only the static binary and config
COPY --from=builder /app/target/aarch64-unknown-linux-musl/release/indexer /app/indexer

# Run the binary
CMD ["/app/indexer"]
