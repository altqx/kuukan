# Kuukan

[![CI](https://github.com/altqx/kuukan/actions/workflows/ci.yml/badge.svg)](https://github.com/altqx/kuukan/actions/workflows/ci.yml)
[![Docker](https://github.com/altqx/kuukan/actions/workflows/docker.yml/badge.svg)](https://github.com/altqx/kuukan/actions/workflows/docker.yml)

Kuukan (空間, "space") is a from-scratch Rust rewrite of
[Jikan REST API v4](https://github.com/jikan-me/jikan-rest): an unofficial
MyAnimeList.net REST API.

It is a drop-in replacement at the HTTP level: the same routes, query
parameters, status codes and error envelopes, so existing Jikan clients work by
pointing their base URL at kuukan. Responses are a *superset* of Jikan's: no
field changed meaning or disappeared, and a few were added. See
[Differences from Jikan](#differences-from-jikan).

Unlike Jikan, Kuukan is a single self-contained binary: SQLite (WAL) for
storage and Tantivy for full-text search. No MongoDB, Redis or Typesense.

## Status

Feature-complete port of Jikan REST v4.2.2 and jikan-php v4.0.12. The port was
verified by a differential harness replaying **270 recorded responses from a
real jikan-rest v4.2.2 instance**: 266 matched exactly (status, headers and
body) and 4 were documented divergences.

That run predates the deliberate divergences listed below, which add fields and
fill in documented ones. It has not been re-run since; the harness lives
outside this repository.

## Differences from Jikan

kuukan no longer treats matching jikan-php as a constraint on its own
behaviour. Everything here is deliberate.

**Routing**

- Everything is served under **`/v1`**; upstream uses `/v4`. Upstream's
  discontinued `/v1`–`/v3` stubs are gone.

**Added fields** — existing fields are untouched, so clients see supersets.

| Endpoint | Field | What it is |
|---|---|---|
| every review endpoint | `scores` | `{overall, story, art, character, enjoyment}`, or `null` where MAL renders no breakdown. jikan-php parses this and never returns it. |
| `/anime/{id}/episodes/{ep}` | `forum_url` | the episode's discussion thread, which the episode *list* already returned |
| `/clubs/{id}` | `pictures` | the picture count MAL shows beside the member count |

**Fuller list items — a fix, not a divergence.** A Jikan resource emits every
key of its shape, `null` included. `/anime/{id}/news`, `/{type}/{id}/forum`,
`/watch/*`, `/recommendations/*` and `/reviews/*` were instead returning each
item exactly as it had been scraped, so a partial scrape produced a partial
response and a stored key outside the shape leaked through. Those endpoints now
go through the item's documented shape, which is what upstream does.

**Other**

- One search tie order is engine-defined and may differ from Typesense's.

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
| `kuukan-core` | Shared vocabulary: `EntityKind`, enums, query parameters, pagination, response envelopes, errors |
| `kuukan-mal` | MyAnimeList client and HTML/JSON parsers (port of jikan-php) |
| `kuukan-store` | SQLite storage, response cache, TTL/expiry, Jikan Mongo importer |
| `kuukan-search` | Tantivy indexes and search query building |
| `kuukan-api` | axum HTTP layer: routes, endpoint cache lifecycle, resources, middleware |
| `kuukan-cli` | `kuukan serve`, indexers, `cache:remove`, `import`, scheduler |

## Development

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

Nothing in the test suite touches the network. `kuukan-mal` reaches
MyAnimeList through one trait with a single method:

```rust
#[async_trait]
pub trait MalSource: Send + Sync {
    async fn get_bytes(&self, url: &str) -> Result<Bytes, MalError>;
}
```

`MalClient` is the HTTP adapter and owns every transport concern (timeout,
proxy, retry, mapping a `>= 400` status onto `BadResponse`). `RecordedSource`,
behind the crate's `testing` feature, replays captured responses from a map and
404s anything it was not given, so an endpoint can be driven end to end with no
network. `crates/kuukan-api/tests/endpoints.rs` does exactly that against the
real router, which is where cache lifecycle, fingerprinting and per-endpoint
TTLs are pinned.

Adding a MAL page means implementing that one method, not a client.

## License

MIT. See `LICENSE` and `NOTICE`.
