-- Kuukan storage schema.
--
-- Replaces the MongoDB collections and Redis cache of jikan-rest with a single
-- SQLite database. Timestamps are unix seconds (INTEGER), JSON payloads are
-- stored as TEXT and queried with the SQLite JSON1 functions.

CREATE TABLE entities (
    kind        TEXT    NOT NULL,
    mal_id      INTEGER NOT NULL,
    payload     TEXT    NOT NULL,
    created_at  INTEGER NOT NULL,
    modified_at INTEGER NOT NULL,
    expires_at  INTEGER,
    PRIMARY KEY (kind, mal_id)
);

CREATE TABLE cache_entries (
    fingerprint TEXT PRIMARY KEY,
    payload     TEXT    NOT NULL,
    created_at  INTEGER NOT NULL,
    modified_at INTEGER NOT NULL,
    expires_at  INTEGER
);

CREATE TABLE source_health (
    id            INTEGER PRIMARY KEY CHECK (id = 1),
    fails         TEXT    NOT NULL DEFAULT '[]',
    failover      INTEGER NOT NULL DEFAULT 0,
    last_downtime INTEGER NOT NULL DEFAULT 0,
    updated_at    INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE search_metrics (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    query      TEXT NOT NULL,
    meta       TEXT,
    created_at INTEGER NOT NULL
);

CREATE TABLE indexer_state (
    name       TEXT PRIMARY KEY,
    cursor     TEXT,
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_entities_kind ON entities (kind);
CREATE INDEX idx_entities_expires_at ON entities (expires_at);
CREATE INDEX idx_cache_entries_expires_at ON cache_entries (expires_at);
CREATE INDEX idx_search_metrics_created_at ON search_metrics (created_at);
