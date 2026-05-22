FROM rust:1.91-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY apps ./apps
COPY packages ./packages

RUN cargo build --release -p philosophy-extractor-api

FROM debian:bookworm-slim

RUN groupadd --system app \
    && useradd --system --gid app --uid 10001 --home-dir /nonexistent --no-create-home app

COPY --from=builder /app/target/release/philosophy-extractor-api /usr/local/bin/philosophy-extractor-api

ENV PHILOSOPHY_EXTRACTOR_API_ADDR=0.0.0.0:8080

EXPOSE 8080

USER app

CMD ["philosophy-extractor-api"]
