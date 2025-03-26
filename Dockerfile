# Build stage
FROM rust:1.85-alpine AS builder

# Install build dependencies
RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    openssl-libs-static \
    pkgconfig \
    build-base \
    perl \
    cmake

# Set environment variables for static linking
ENV OPENSSL_STATIC=1
ENV OPENSSL_LIB_DIR=/usr/lib
ENV OPENSSL_INCLUDE_DIR=/usr/include

# Create a new empty shell project
WORKDIR /usr/src/decalid
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
COPY .sqlx ./.sqlx

# Build the application
RUN --mount=type=cache,target=/usr/src/decalid/target/ \
    --mount=type=cache,target=/usr/local/cargo/git/db \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    mkdir -p /usr/src/decalid/dist && \
    cargo build --release && \
    cp /usr/src/decalid/target/release/decalid-rs /usr/src/decalid/dist/decalid-rs

# Runtime stage
FROM alpine:3.19

# Install runtime dependencies
RUN apk add --no-cache ca-certificates sqlite libgcc openssl

# Create app directory
WORKDIR /app

# Copy the binary from the builder stage
COPY --from=builder /usr/src/decalid/dist/decalid-rs /app/decalid-rs

# Copy configuration
COPY config.json /app/

# Create volume for persistent data
VOLUME /app/data

# Create entrypoint script
COPY docker-entrypoint.sh /app/
RUN chmod +x /app/docker-entrypoint.sh

# Set environment variables
ENV DATABASE_URL=sqlite:///app/data/db.sqlite

# Expose the server port
EXPOSE 8080

# Set the entrypoint
ENTRYPOINT ["/app/docker-entrypoint.sh"]

# Default command
CMD ["server"]
