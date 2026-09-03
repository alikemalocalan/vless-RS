# Multi-stage production build for Railway & Docker deployments
FROM rust:latest AS builder

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

# Install official Xray-core for REALITY TLS 1.3 engine
RUN ARCH=$(uname -m) && \
    case "$ARCH" in \
      x86_64) XRAY_ARCH="64" ;; \
      aarch64) XRAY_ARCH="arm64-v8a" ;; \
      *) XRAY_ARCH="64" ;; \
    esac && \
    curl -sL "https://github.com/XTLS/Xray-core/releases/latest/download/Xray-linux-${XRAY_ARCH}.zip" -o /tmp/xray.zip && \
    unzip -q /tmp/xray.zip -d /usr/local/bin xray && \
    chmod +x /usr/local/bin/xray && \
    rm -rf /tmp/xray.zip

WORKDIR /app

COPY --from=builder /usr/src/vless-RS/target/release/vless-RS /app/vless-RS

ENV PORT=8080
ENV BIND=0.0.0.0
ENV DEST=gateway.icloud.com:443
ENV SNI=gateway.icloud.com

EXPOSE 8080

ENTRYPOINT ["/app/vless-RS"]
