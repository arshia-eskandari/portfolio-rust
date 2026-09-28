# Build stage
FROM rust:1.90-slim AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY templates ./templates
COPY examples ./examples
RUN cargo build --release --locked

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/portfolio /usr/local/bin/portfolio
COPY static ./static
ENV APP_HOST=0.0.0.0 APP_PORT=3000
EXPOSE 3000
HEALTHCHECK --interval=30s --timeout=3s CMD curl -fsS http://localhost:3000/health || exit 1
USER nobody
CMD ["portfolio"]
