# Builder stage
FROM rust:1.84.1-alpine3.21 as builder

EXPOSE 8080

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

# Add x86_64-musl target
RUN rustup target add x86_64-unknown-linux-musl

# Copy source code
COPY Cargo.toml ./
COPY indexer indexer/
COPY indexer_database indexer_database/

# Build statically linked release for amd64
ENV RUSTFLAGS='-C target-feature=+crt-static'
RUN cargo build --release --bin indexer --target x86_64-unknown-linux-musl

# Runtime stage - using Alpine instead of scratch for SSL certificates
FROM alpine:3.21
EXPOSE 8080
# Install SSL certificates
RUN apk add --no-cache ca-certificates

WORKDIR /app

# Copy only the static binary and config
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/indexer /app/indexer
COPY .env /app/.env
# Run the binary
CMD ["/app/indexer"]