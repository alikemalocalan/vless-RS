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
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /usr/src/vless-RS/target/release/vless-RS /app/vless-RS

ENV PORT=8080
ENV BIND=0.0.0.0
ENV DEST=gateway.icloud.com:443
ENV SNI=gateway.icloud.com

EXPOSE 8080

ENTRYPOINT ["/app/vless-RS"]
