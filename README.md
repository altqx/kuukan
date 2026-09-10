# Kuukan

[![CI](https://github.com/altqx/kuukan/actions/workflows/ci.yml/badge.svg)](https://github.com/altqx/kuukan/actions/workflows/ci.yml)
[![Docker](https://github.com/altqx/kuukan/actions/workflows/docker.yml/badge.svg)](https://github.com/altqx/kuukan/actions/workflows/docker.yml)

Kuukan (空間, "space") is a from-scratch Rust rewrite of
[Jikan REST API v4](https://github.com/jikan-me/jikan-rest): an unofficial
MyAnimeList.net REST API.

It is a drop-in replacement at the HTTP level: identical routes, query
parameters, JSON responses, status codes and error envelopes, so existing
Jikan clients work by pointing their base URL at kuukan — with one deliberate
difference: kuukan serves everything under **`/v1`** (upstream uses `/v4`).

Unlike Jikan, Kuukan is a single self-contained binary: SQLite (WAL) for
storage and Tantivy for full-text search. No MongoDB, Redis or Typesense.

## Status

Feature-complete port of Jikan REST v4.2.2 and jikan-php v4.0.12, verified by a
differential harness that replays **270 recorded responses from a real
jikan-rest v4.2.2 instance**: 266 match exactly (status, headers and body) and
4 are documented divergences (kuukan serves only `/v1`, so upstream's
discontinued `/v1`–`/v3` stubs are gone; one search tie order is
engine-defined).

## Quick start

```bash
cargo build --release
# optional: crawl data into the local store (uses purarue/mal-id-cache)
./target/release/kuukan indexer anime --delay 1
# serve the API on /v1
./target/release/kuukan serve --listen 0.0.0.0:8080
```

Data lives in SQLite + Tantivy (`KUUKAN_DB_PATH`, `KUUKAN_SEARCH_PATH`), so an
existing Jikan MongoDB cache can be exported to JSON and loaded with
`kuukan import` without re-scraping MyAnimeList.

## Docker

Multi-arch images (`linux/amd64`, `linux/arm64`) are published to
[GHCR](https://github.com/altqx/kuukan/pkgs/container/kuukan):

| Tag | Published from |
|---|---|
| `0.1.0`, `0.1`, `latest` | `v*` release tags |
| `edge` | every push to `main` |
| `sha-<short>` | every image build |

```bash
docker run -d --name kuukan -p 8080:8080 -v kuukan-data:/app/data ghcr.io/altqx/kuukan:0.1.0
```

Or with Compose, which pulls the published image by default and falls back to
building from source (`docker compose up -d --build` forces a local build):

```bash
docker compose up -d
curl http://127.0.0.1:8080/v1/anime/1
```

## Releases

Every `v*` tag runs the [release workflow](.github/workflows/release.yml),
which

1. builds the x86_64 Linux binary and publishes it (with checksums) to
   [GitHub Releases](https://github.com/altqx/kuukan/releases), and
2. publishes the matching container image to GHCR via
   [docker.yml](.github/workflows/docker.yml).

## Layout

| Crate | Purpose |
|---|---|
| `kuukan-core` | Domain models, API DTOs, enums, query parameters, errors |
| `kuukan-mal` | MyAnimeList client and HTML/JSON parsers (port of jikan-php) |
| `kuukan-store` | SQLite storage, response cache, TTL/expiry, Jikan Mongo importer |
| `kuukan-search` | Tantivy indexes and search query building |
| `kuukan-api` | axum HTTP layer: routes, resources, middleware |
| `kuukan-cli` | `kuukan serve`, indexers, `cache:remove`, `import`, scheduler |

## License

MIT. See `LICENSE` and `NOTICE`.
