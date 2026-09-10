# syntax=docker/dockerfile:1
FROM rust:1.98-slim-bookworm AS build
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libxml2-dev clang libclang-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /src
COPY . .
RUN cargo build --release --locked --bin kuukan

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libxml2 && rm -rf /var/lib/apt/lists/* \
    && useradd --system --home /app --shell /usr/sbin/nologin kuukan
WORKDIR /app
COPY --from=build /src/target/release/kuukan /usr/local/bin/kuukan
RUN mkdir -p /app/data && chown -R kuukan:kuukan /app
USER kuukan
ENV KUUKAN_DB_PATH=/app/data/kuukan.db \
    KUUKAN_SEARCH_PATH=/app/data/search \
    RUST_LOG=info
EXPOSE 8080
VOLUME ["/app/data"]
ENTRYPOINT ["kuukan"]
CMD ["serve", "--listen", "0.0.0.0:8080"]
