# Multi-stage production build for Railway & Docker deployments
FROM rust:bookworm AS builder

WORKDIR /usr/src/vless-RS

# Pre-cache dependencies
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src

# Copy source code and build production binary
COPY src ./src
RUN touch src/main.rs && cargo build --release

# Final lightweight runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    unzip \
    && rm -rf /var/lib/apt/lists/*

# Install pure Rust shoes engine (Ultra-low ~12MB RAM, VLESS + REALITY + Vision)
RUN ARCH=$(uname -m) && \
    case "$ARCH" in \
      x86_64) SHOES_ARCH="x86_64" ;; \
      aarch64) SHOES_ARCH="aarch64" ;; \
      *) SHOES_ARCH="x86_64" ;; \
    esac && \
    curl -sL "https://github.com/cfal/shoes/releases/download/v0.2.7/shoes-${SHOES_ARCH}-unknown-linux-musl.tar.gz" -o /tmp/shoes.tar.gz && \
    tar -xzf /tmp/shoes.tar.gz -C /usr/local/bin && \
    chmod +x /usr/local/bin/shoes && \
    rm -rf /tmp/shoes.tar.gz

WORKDIR /app

COPY --from=builder /usr/src/vless-RS/target/release/vless-RS /app/vless-RS

ENV PORT=8080
ENV BIND=0.0.0.0
ENV DEST=gateway.icloud.com:443
ENV SNI=gateway.icloud.com

EXPOSE 8080

ENTRYPOINT ["/app/vless-RS"]
