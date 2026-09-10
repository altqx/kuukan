//! Bulk import of JSON dumps, replacing the "import a raw MongoDB dump" step
//! of a jikan-rest deployment.
//!
//! # Expected format
//!
//! ```json
//! {
//!   "anime": [
//!     { "mal_id": 1, "title": "Cowboy Bebop", "...": "..." },
//!     { "mal_id": 2, "title": "Trigun", "...": "..." }
//!   ],
//!   "manga": {
//!     "13": { "title": "One Piece", "...": "..." }
//!   },
//!   "characters": [ { "mal_id": 1, "name": "Spike Spiegel", "...": "..." } ]
//! }
//! ```
//!
//! * The top-level JSON value is an object mapping entity kinds to document
//!   collections. Kind keys are either the canonical [`EntityKind`] strings
//!   (`anime`, `manga`, `character`, `person`, `user`, `club`, `producer`,
//!   `magazine`, `genre_anime`, `genre_manga`, `episode`) or the historical
//!   jikan-rest MongoDB collection names (`characters`, `people`, `clubs`,
//!   `producers`, `magazines`, `genres_anime`, `genres_manga`,
//!   `anime_episode`).
//! * Each collection is either an array of complete API-shaped documents or an
//!   object mapping `mal_id` strings to documents.
//! * `mal_id` is read from the document field when present (`1` or `"1"`),
//!   otherwise from the object key. A document may also carry its own `kind`
//!   field, which takes precedence over the collection key.
//! * Documents whose kind or `mal_id` cannot be resolved are counted in
//!   [`ImportReport::skipped`] instead of failing the import.
//!
//! Imported entities are written without a TTL (`expires_at = NULL`); existing
//! rows are updated but keep their `created_at`. The whole import runs in a
//! single transaction, so a failure leaves the database untouched.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value;

use crate::db::{now_unix, Store, StoreError};
use crate::entities::{upsert_entity_exec, EntityKind, StoredEntity};

/// Summary of an [`Store::import_json_dump`] run.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ImportReport {
    /// Number of entities written (inserted or updated).
    pub imported: u64,
    /// Number of documents without a resolvable kind/`mal_id`.
    pub skipped: u64,
    /// Imported counts keyed by canonical [`EntityKind`] string.
    pub by_kind: BTreeMap<String, u64>,
}

impl ImportReport {
    fn record(&mut self, kind: EntityKind) {
        self.imported += 1;
        *self.by_kind.entry(kind.as_str().to_string()).or_insert(0) += 1;
    }
}

impl Store {
    /// Import a JSON dump from `path`. See the module documentation for the
    /// expected shape.
    pub async fn import_json_dump(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<ImportReport, StoreError> {
        let bytes = std::fs::read(path)?;
        let root: Value = serde_json::from_slice(&bytes)?;
        self.import_json_value(root).await
    }

    /// Import a JSON dump already parsed into a [`Value`].
    pub async fn import_json_value(&self, root: Value) -> Result<ImportReport, StoreError> {
        let root = match root {
            Value::Object(root) => root,
            _ => {
                return Err(StoreError::InvalidPayload(
                    "import dump must be a JSON object of kind -> documents".to_string(),
                ))
            }
        };

        let now = now_unix();
        let mut transaction = self.pool().begin().await?;
        let mut report = ImportReport::default();

        for (collection, documents) in root {
            let collection_kind = EntityKind::from_dump_key(&collection);
            match documents {
                Value::Array(items) => {
                    for item in items {
                        match import_document(item, collection_kind, None, now) {
                            Some(entity) => {
                                upsert_entity_exec(&mut transaction, &entity, None, now).await?;
                                report.record(entity.kind);
                            }
                            None => report.skipped += 1,
                        }
                    }
                }
                Value::Object(map) => {
                    for (key, item) in map {
                        match import_document(item, collection_kind, Some(&key), now) {
                            Some(entity) => {
                                upsert_entity_exec(&mut transaction, &entity, None, now).await?;
                                report.record(entity.kind);
                            }
                            None => report.skipped += 1,
                        }
                    }
                }
                _ => {
                    return Err(StoreError::InvalidPayload(format!(
                        "import collection `{collection}` must be an array or an object"
                    )))
                }
            }
        }

        transaction.commit().await?;
        Ok(report)
    }
}

/// Resolve one dump document into a [`StoredEntity`], or `None` when the kind
/// or `mal_id` is missing/unparsable.
fn import_document(
    document: Value,
    collection_kind: Option<EntityKind>,
    map_key: Option<&str>,
    now: i64,
) -> Option<StoredEntity> {
    let object = document.as_object()?;

    let kind = object
        .get("kind")
        .and_then(Value::as_str)
        .and_then(EntityKind::from_dump_key)
        .or(collection_kind)?;

    let mal_id = object
        .get("mal_id")
        .and_then(value_as_i64)
        .or_else(|| map_key.and_then(|key| key.trim().parse::<i64>().ok()))?;

    Some(StoredEntity {
        kind,
        mal_id,
        payload: document,
        created_at: now,
        modified_at: now,
        expires_at: None,
    })
}

/// Accept both JSON numbers and numeric strings for `mal_id`.
fn value_as_i64(value: &Value) -> Option<i64> {
    value.as_i64().or_else(|| {
        value
            .as_str()
            .and_then(|text| text.trim().parse::<i64>().ok())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn import_json_dump_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("dump.json");
        let dump = json!({
            "anime": [
                {"mal_id": 1, "title": "Cowboy Bebop"},
                {"mal_id": "2", "title": "Trigun"}
            ],
            "manga": {
                "13": {"title": "One Piece"},
                "not-a-number": {"mal_id": 14, "title": "Berserk"}
            },
            "characters": [
                {"mal_id": 5, "name": "Spike Spiegel"}
            ],
            "unknown_collection": [
                {"mal_id": 99}
            ]
        });
        std::fs::write(&path, serde_json::to_vec(&dump).expect("encode")).expect("write");

        let store = Store::open_in_memory().await.expect("open");
        let report = store.import_json_dump(&path).await.expect("import");
        assert_eq!(report.imported, 5);
        assert_eq!(report.skipped, 1);
        assert_eq!(report.by_kind["anime"], 2);
        assert_eq!(report.by_kind["manga"], 2);
        assert_eq!(report.by_kind["character"], 1);

        let anime = store
            .get_entity(EntityKind::Anime, 2)
            .await
            .expect("get")
            .expect("anime 2 imported from string mal_id");
        assert_eq!(anime.payload["title"], "Trigun");
        assert_eq!(
            store.entity_count(EntityKind::Anime).await.expect("count"),
            2
        );

        let manga = store
            .get_entity(EntityKind::Manga, 13)
            .await
            .expect("get")
            .expect("manga 13 imported from object key");
        assert_eq!(manga.payload["title"], "One Piece");

        let character = store
            .get_entity(EntityKind::Character, 5)
            .await
            .expect("get")
            .expect("character imported from mongo collection name");
        assert_eq!(character.payload["name"], "Spike Spiegel");
        assert_eq!(character.expires_at, None);

        // Importing the same dump again updates rows instead of duplicating.
        let report = store.import_json_dump(&path).await.expect("re-import");
        assert_eq!(report.imported, 5);
        assert_eq!(
            store.entity_count(EntityKind::Anime).await.expect("count"),
            2
        );
    }

    #[tokio::test]
    async fn import_preserves_created_at_of_existing_rows() {
        let store = Store::open_in_memory().await.expect("open");
        store
            .upsert_entity_at(
                &StoredEntity::new(EntityKind::Anime, 1, json!({"mal_id": 1, "old": true})),
                Some(10),
                100,
            )
            .await
            .expect("seed");

        let report = store
            .import_json_value(json!({
                "anime": [{"mal_id": 1, "title": "Fresh"}]
            }))
            .await
            .expect("import");
        assert_eq!(report.imported, 1);

        let imported = store
            .get_entity(EntityKind::Anime, 1)
            .await
            .expect("get")
            .expect("present");
        assert_eq!(imported.created_at, 100, "created_at survives the import");
        assert_eq!(imported.expires_at, None);
        assert_eq!(imported.payload["title"], "Fresh");
        assert!(imported.modified_at >= 100);
    }

    #[tokio::test]
    async fn import_rejects_non_object_roots() {
        let store = Store::open_in_memory().await.expect("open");
        let error = store
            .import_json_value(json!([{"mal_id": 1}]))
            .await
            .expect_err("must reject arrays");
        assert!(matches!(error, StoreError::InvalidPayload(_)));
    }

    #[tokio::test]
    async fn import_reports_missing_files() {
        let store = Store::open_in_memory().await.expect("open");
        let error = store
            .import_json_dump("/definitely/not/a/real/dump.json")
            .await
            .expect_err("must fail");
        assert!(matches!(error, StoreError::Io(_)));
    }
}
