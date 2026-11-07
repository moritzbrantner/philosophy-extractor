FROM rust:1.99-bookworm@sha256:e6f2cff35d0f309acf8433b4f05878ed77092602c5065cc81801177da0bb3a9c AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY apps ./apps
COPY packages ./packages

RUN cargo build --release -p philosophy-extractor-api

FROM debian:bookworm-slim@sha256:734a72b3d6142a71993b4bc580ca01d8ca687f77a2c422d96a961d3376b179bc

RUN groupadd --system app \
    && useradd --system --gid app --uid 10001 --home-dir /nonexistent --no-create-home app

COPY --from=builder /app/target/release/philosophy-extractor-api /usr/local/bin/philosophy-extractor-api

ENV PHILOSOPHY_EXTRACTOR_API_ADDR=0.0.0.0:8080

EXPOSE 8080

USER app

CMD ["philosophy-extractor-api"]
