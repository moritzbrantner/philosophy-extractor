FROM rust:1.91-bookworm@sha256:c1e5f19e773b7878c3f7a805dd00a495e747acbdc76fb2337a4ebf0418896b33 AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY apps ./apps
COPY packages ./packages

RUN cargo build --release -p philosophy-extractor-api

FROM debian:bookworm-slim@sha256:99c0b25c46d0214d5a2091092e7908907dcaecd6a962cde0ac700acee222b6f3

RUN groupadd --system app \
    && useradd --system --gid app --uid 10001 --home-dir /nonexistent --no-create-home app

COPY --from=builder /app/target/release/philosophy-extractor-api /usr/local/bin/philosophy-extractor-api

ENV PHILOSOPHY_EXTRACTOR_API_ADDR=0.0.0.0:8080

EXPOSE 8080

USER app

CMD ["philosophy-extractor-api"]
