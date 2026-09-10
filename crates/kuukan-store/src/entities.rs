//! Entity storage (`entities` table) and the [`EntityKind`] vocabulary.
//!
//! This replaces the per-model MongoDB collections of jikan-rest (`anime`,
//! `manga`, `characters`, `people`, ...). Every scraped, API-shaped document is
//! stored in one table, keyed by `(kind, mal_id)`, with the document itself in
//! a JSON `payload` column.
//!
//! Timestamps are unix seconds. `created_at` is set on the first insert and
//! never changes; `modified_at` is bumped on every upsert; `expires_at` is
//! `modified_at + ttl` when the caller supplies a TTL and `NULL` (never
//! expires) otherwise. Expiry is informational: `get_entity` returns expired
//! documents as well, because jikan-rest's `DefaultCachedScraperService`
//! decides when to re-scrape (and can serve stale data while MAL is down).

use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::SqliteConnection;

use crate::db::{now_unix, Store, StoreError};

/// The kind of an entity, matching the MongoDB collections of jikan-rest.
///
/// The serde representation is the snake_case string used in the `kind`
/// column: `anime`, `manga`, `character`, `person`, `user`, `club`,
/// `producer`, `magazine`, `genre_anime`, `genre_manga`, `episode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    Anime,
    Manga,
    Character,
    Person,
    User,
    Club,
    Producer,
    Magazine,
    GenreAnime,
    GenreManga,
    Episode,
}

impl EntityKind {
    /// Every kind, in declaration order.
    pub const ALL: [EntityKind; 11] = [
        EntityKind::Anime,
        EntityKind::Manga,
        EntityKind::Character,
        EntityKind::Person,
        EntityKind::User,
        EntityKind::Club,
        EntityKind::Producer,
        EntityKind::Magazine,
        EntityKind::GenreAnime,
        EntityKind::GenreManga,
        EntityKind::Episode,
    ];

    /// The canonical string value stored in the `kind` column.
    pub const fn as_str(self) -> &'static str {
        match self {
            EntityKind::Anime => "anime",
            EntityKind::Manga => "manga",
            EntityKind::Character => "character",
            EntityKind::Person => "person",
            EntityKind::User => "user",
            EntityKind::Club => "club",
            EntityKind::Producer => "producer",
            EntityKind::Magazine => "magazine",
            EntityKind::GenreAnime => "genre_anime",
            EntityKind::GenreManga => "genre_manga",
            EntityKind::Episode => "episode",
        }
    }

    /// Parse a canonical kind string or a historical jikan-rest collection
    /// name.
    ///
    /// [`FromStr`] accepts only the canonical values (the serde contract).
    /// `from_dump_key` additionally accepts the MongoDB collection names used
    /// before the SQLite rewrite (`characters`, `people`, `clubs`,
    /// `producers`, `magazines`, `genres_anime`, `genres_manga`,
    /// `anime_episode`), which makes Mongo-derived JSON dumps importable.
    pub fn from_dump_key(key: &str) -> Option<Self> {
        let normalized = key.trim().to_ascii_lowercase();
        if let Ok(kind) = normalized.parse::<EntityKind>() {
            return Some(kind);
        }
        match normalized.as_str() {
            "characters" => Some(EntityKind::Character),
            "people" => Some(EntityKind::Person),
            "users" | "profiles" => Some(EntityKind::User),
            "clubs" => Some(EntityKind::Club),
            "producers" => Some(EntityKind::Producer),
            "magazines" => Some(EntityKind::Magazine),
            "genres_anime" => Some(EntityKind::GenreAnime),
            "genres_manga" => Some(EntityKind::GenreManga),
            "anime_episode" | "anime_episodes" | "episodes" => Some(EntityKind::Episode),
            _ => None,
        }
    }
}

impl std::fmt::Display for EntityKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for EntityKind {
    type Err = StoreError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "anime" => Ok(EntityKind::Anime),
            "manga" => Ok(EntityKind::Manga),
            "character" => Ok(EntityKind::Character),
            "person" => Ok(EntityKind::Person),
            "user" => Ok(EntityKind::User),
            "club" => Ok(EntityKind::Club),
            "producer" => Ok(EntityKind::Producer),
            "magazine" => Ok(EntityKind::Magazine),
            "genre_anime" => Ok(EntityKind::GenreAnime),
            "genre_manga" => Ok(EntityKind::GenreManga),
            "episode" => Ok(EntityKind::Episode),
            other => Err(StoreError::InvalidKind(other.to_string())),
        }
    }
}

/// A stored entity: one API-shaped document plus its cache timestamps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredEntity {
    /// Entity kind (table partition).
    pub kind: EntityKind,
    /// MAL id, unique within `kind`.
    pub mal_id: i64,
    /// Complete API-shaped JSON document.
    pub payload: Value,
    /// Unix seconds of the first insert (never changed by upserts).
    pub created_at: i64,
    /// Unix seconds of the last write.
    pub modified_at: i64,
    /// Unix seconds after which the document is stale, or `None` for never.
    pub expires_at: Option<i64>,
}

impl StoredEntity {
    /// Build an entity with `created_at = modified_at = now` and no expiry.
    ///
    /// The timestamp fields are placeholders: [`Store::upsert_entity`] manages
    /// them and ignores whatever the caller passes.
    pub fn new(kind: EntityKind, mal_id: i64, payload: Value) -> Self {
        let now = now_unix();
        Self {
            kind,
            mal_id,
            payload,
            created_at: now,
            modified_at: now,
            expires_at: None,
        }
    }

    /// Whether the entity is stale at `now`.
    ///
    /// Mirrors `CachedData::isExpired()`: an entity is expired when
    /// `now > expires_at`. `None` means never expired.
    pub fn is_expired(&self, now: i64) -> bool {
        matches!(self.expires_at, Some(expires_at) if now > expires_at)
    }
}

#[derive(sqlx::FromRow)]
struct EntityRow {
    kind: String,
    mal_id: i64,
    payload: String,
    created_at: i64,
    modified_at: i64,
    expires_at: Option<i64>,
}

impl TryFrom<EntityRow> for StoredEntity {
    type Error = StoreError;

    fn try_from(row: EntityRow) -> Result<Self, StoreError> {
        Ok(StoredEntity {
            kind: row.kind.parse()?,
            mal_id: row.mal_id,
            payload: serde_json::from_str(&row.payload)?,
            created_at: row.created_at,
            modified_at: row.modified_at,
            expires_at: row.expires_at,
        })
    }
}

/// SQL for the non-adult (`sfw`) filter, ported from
/// `JikanApiModel::scopeRandom` (`app/JikanApiModel.php`).
///
/// MongoDB matched array membership with `$nin`/`$ne`; the same semantics on
/// the JSON payload are:
///
/// * none of `genres`, `explicit_genres`, `themes` or `demographics` contains
///   a `mal_id` of `Hentai` (12) or `Erotica` (49),
/// * `rating != "Rx - Hentai"` (a missing `rating` passes, like Mongo `$ne`),
/// * `type != "Doujinshi"` (the `MangaTypeEnum::doujin()` label).
///
/// The PHP `$sfwFilter` only names `demographics.mal_id` (12/49) and
/// `genres.mal_id` (12). Jikan v4 stores adult genres in `explicit_genres`
/// and `themes` too, so those arrays are filtered as well: stricter than the
/// literal PHP query, but it matches the intent of the `sfw` parameter.
/// Mirrors `JikanApiModel::scopeRandom()` exactly:
/// `demographics.mal_id $nin [12, 49]`, `rating $ne 'Rx - Hentai'`,
/// `type $ne 'Doujinshi'`, `genres.mal_id $nin [12]`.
/// (The PHP query does *not* inspect explicit_genres or themes.)
const SFW_FILTER_SQL: &str = " AND json_extract(payload, '$.rating') IS NOT 'Rx - Hentai' \
     AND json_extract(payload, '$.type') IS NOT 'Doujinshi' \
     AND NOT EXISTS (SELECT 1 FROM json_each(json_extract(payload, '$.genres')) \
                     WHERE json_extract(value, '$.mal_id') IN (12)) \
     AND NOT EXISTS (SELECT 1 FROM json_each(json_extract(payload, '$.demographics')) \
                     WHERE json_extract(value, '$.mal_id') IN (12, 49))";

/// `approved = false` filter for random endpoints (`$unapproved`).
///
/// JSON `false` extracts as integer `0`; a missing or null `approved` field
/// yields SQL `NULL`, which does not equal `0` (same as Mongo `approved: false`
/// not matching documents without the field).
const UNAPPROVED_FILTER_SQL: &str = " AND json_extract(payload, '$.approved') = 0";

/// Insert-or-update an entity on an explicit connection.
///
/// On insert `created_at = modified_at = now`; on conflict the existing
/// `created_at` is preserved and `modified_at` is set to `now`. `expires_at`
/// is `now + ttl` when a TTL is given, `NULL` otherwise.
pub(crate) async fn upsert_entity_exec(
    connection: &mut SqliteConnection,
    entity: &StoredEntity,
    ttl_secs: Option<i64>,
    now: i64,
) -> Result<(), StoreError> {
    let payload = serde_json::to_string(&entity.payload)?;
    let expires_at = ttl_secs.map(|ttl| now.saturating_add(ttl));
    sqlx::query(
        "INSERT INTO entities (kind, mal_id, payload, created_at, modified_at, expires_at) \
         VALUES (?, ?, ?, ?, ?, ?) \
         ON CONFLICT(kind, mal_id) DO UPDATE SET \
             payload = excluded.payload, \
             modified_at = excluded.modified_at, \
             expires_at = excluded.expires_at",
    )
    .bind(entity.kind.as_str())
    .bind(entity.mal_id)
    .bind(payload)
    .bind(now)
    .bind(now)
    .bind(expires_at)
    .execute(&mut *connection)
    .await?;
    Ok(())
}

impl Store {
    /// Fetch one entity, including expired ones.
    pub async fn get_entity(
        &self,
        kind: EntityKind,
        mal_id: i64,
    ) -> Result<Option<StoredEntity>, StoreError> {
        let row = sqlx::query_as::<_, EntityRow>(
            "SELECT kind, mal_id, payload, created_at, modified_at, expires_at \
             FROM entities WHERE kind = ? AND mal_id = ?",
        )
        .bind(kind.as_str())
        .bind(mal_id)
        .fetch_optional(self.pool())
        .await?;
        row.map(StoredEntity::try_from).transpose()
    }

    /// Insert or update an entity, letting the store manage timestamps.
    ///
    /// Timestamps on `entity` are ignored: on insert `created_at` and
    /// `modified_at` become the current time; on update `created_at` is kept
    /// and `modified_at` becomes the current time. `expires_at` becomes
    /// `modified_at + ttl_secs` when a TTL is given, otherwise `NULL`.
    pub async fn upsert_entity(
        &self,
        entity: StoredEntity,
        ttl_secs: Option<i64>,
    ) -> Result<(), StoreError> {
        self.upsert_entity_at(&entity, ttl_secs, now_unix()).await
    }

    /// Timestamp-injecting variant of [`Store::upsert_entity`], for
    /// deterministic tests.
    pub(crate) async fn upsert_entity_at(
        &self,
        entity: &StoredEntity,
        ttl_secs: Option<i64>,
        now: i64,
    ) -> Result<(), StoreError> {
        let mut connection = self.pool().acquire().await?;
        upsert_entity_exec(&mut connection, entity, ttl_secs, now).await
    }

    /// Delete one entity. Returns `true` when a row was removed.
    pub async fn delete_entity(&self, kind: EntityKind, mal_id: i64) -> Result<bool, StoreError> {
        let result = sqlx::query("DELETE FROM entities WHERE kind = ? AND mal_id = ?")
            .bind(kind.as_str())
            .bind(mal_id)
            .execute(self.pool())
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// List entities of one kind ordered by `mal_id`, with pagination.
    ///
    /// Negative `offset`/`limit` values are clamped to 0 (and `LIMIT 0` yields
    /// no rows), so a malformed caller cannot turn into `LIMIT -1` (unbounded)
    /// in SQLite.
    pub async fn list_entities(
        &self,
        kind: EntityKind,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<StoredEntity>, StoreError> {
        let rows = sqlx::query_as::<_, EntityRow>(
            "SELECT kind, mal_id, payload, created_at, modified_at, expires_at \
             FROM entities WHERE kind = ? ORDER BY mal_id LIMIT ? OFFSET ?",
        )
        .bind(kind.as_str())
        .bind(limit.max(0))
        .bind(offset.max(0))
        .fetch_all(self.pool())
        .await?;
        rows.into_iter().map(StoredEntity::try_from).collect()
    }

    /// Number of stored entities of one kind.
    pub async fn entity_count(&self, kind: EntityKind) -> Result<i64, StoreError> {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM entities WHERE kind = ?")
            .bind(kind.as_str())
            .fetch_one(self.pool())
            .await?;
        Ok(count)
    }

    /// Pick up to `n` random entities of one kind, mirroring
    /// `JikanApiModel::scopeRandom` for the `sfw`/`unapproved` filters.
    ///
    /// * `sfw = true` excludes hentai/erotica genres, themes and demographics,
    ///   the `Rx - Hentai` rating and the `Doujinshi` type.
    /// * `unapproved = true` keeps only documents whose `approved` field is
    ///   `false`.
    ///
    /// `n <= 0` returns an empty list.
    pub async fn random_entities(
        &self,
        kind: EntityKind,
        n: i64,
        sfw: bool,
        unapproved: bool,
    ) -> Result<Vec<StoredEntity>, StoreError> {
        if n <= 0 {
            return Ok(Vec::new());
        }

        let mut sql = String::from(
            "SELECT kind, mal_id, payload, created_at, modified_at, expires_at \
             FROM entities WHERE kind = ?",
        );
        if sfw {
            sql.push_str(SFW_FILTER_SQL);
        }
        if unapproved {
            sql.push_str(UNAPPROVED_FILTER_SQL);
        }
        sql.push_str(" ORDER BY RANDOM() LIMIT ?");

        // The only dynamic parts above are compile-time constant fragments;
        // `kind` and `n` are bound parameters.
        let rows = sqlx::query_as::<_, EntityRow>(sqlx::AssertSqlSafe(sql))
            .bind(kind.as_str())
            .bind(n)
            .fetch_all(self.pool())
            .await?;
        rows.into_iter().map(StoredEntity::try_from).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn anime(mal_id: i64, payload: Value) -> StoredEntity {
        StoredEntity::new(EntityKind::Anime, mal_id, payload)
    }

    #[tokio::test]
    async fn entity_crud_and_expiry() {
        let store = Store::open_in_memory().await.expect("open");
        let payload = json!({"mal_id": 1, "title": "Cowboy Bebop"});

        store
            .upsert_entity(anime(1, payload.clone()), Some(60))
            .await
            .expect("upsert");

        let stored = store
            .get_entity(EntityKind::Anime, 1)
            .await
            .expect("get")
            .expect("present");
        assert_eq!(stored.payload, payload);
        assert_eq!(stored.created_at, stored.modified_at);
        assert_eq!(stored.expires_at, Some(stored.modified_at + 60));
        assert!(!stored.is_expired(stored.modified_at + 60));
        assert!(stored.is_expired(stored.modified_at + 61));
        assert!(store
            .get_entity(EntityKind::Manga, 1)
            .await
            .expect("get")
            .is_none());

        store
            .upsert_entity(anime(2, json!({"mal_id": 2})), None)
            .await
            .expect("upsert");
        assert_eq!(
            store.entity_count(EntityKind::Anime).await.expect("count"),
            2
        );

        let listed = store
            .list_entities(EntityKind::Anime, 0, 10)
            .await
            .expect("list");
        assert_eq!(
            listed
                .iter()
                .map(|entity| entity.mal_id)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );

        assert!(store
            .delete_entity(EntityKind::Anime, 1)
            .await
            .expect("delete"));
        assert!(!store
            .delete_entity(EntityKind::Anime, 1)
            .await
            .expect("delete"));
        assert_eq!(
            store.entity_count(EntityKind::Anime).await.expect("count"),
            1
        );
    }

    #[tokio::test]
    async fn upsert_sets_and_preserves_timestamps() {
        let store = Store::open_in_memory().await.expect("open");

        store
            .upsert_entity_at(
                &anime(5, json!({"mal_id": 5, "revision": 1})),
                Some(100),
                1_000,
            )
            .await
            .expect("insert");
        let inserted = store
            .get_entity(EntityKind::Anime, 5)
            .await
            .expect("get")
            .expect("present");
        assert_eq!(inserted.created_at, 1_000);
        assert_eq!(inserted.modified_at, 1_000);
        assert_eq!(inserted.expires_at, Some(1_100));

        store
            .upsert_entity_at(&anime(5, json!({"mal_id": 5, "revision": 2})), None, 2_000)
            .await
            .expect("update");
        let updated = store
            .get_entity(EntityKind::Anime, 5)
            .await
            .expect("get")
            .expect("present");
        assert_eq!(updated.created_at, 1_000, "created_at is kept on update");
        assert_eq!(updated.modified_at, 2_000);
        assert_eq!(updated.expires_at, None);
        assert_eq!(updated.payload["revision"], 2);
    }

    #[tokio::test]
    async fn random_entities_apply_sfw_and_unapproved_filters() {
        let store = Store::open_in_memory().await.expect("open");

        let docs = [
            // 1: safe, approved.
            (
                1_i64,
                json!({
                    "mal_id": 1, "rating": "G - All Ages", "type": "TV", "approved": true,
                    "genres": [{"mal_id": 1, "name": "Action"}],
                    "explicit_genres": [], "themes": [],
                    "demographics": [{"mal_id": 27, "name": "Shounen"}]
                }),
            ),
            // 2: hentai listed in explicit_genres.
            (
                2,
                json!({
                    "mal_id": 2, "rating": "G - All Ages", "type": "TV", "approved": true,
                    "genres": [], "explicit_genres": [{"mal_id": 12, "name": "Hentai"}],
                    "themes": [], "demographics": []
                }),
            ),
            // 3: erotica listed in themes.
            (
                3,
                json!({
                    "mal_id": 3, "rating": "G - All Ages", "type": "TV", "approved": true,
                    "genres": [], "explicit_genres": [],
                    "themes": [{"mal_id": 49, "name": "Erotica"}], "demographics": []
                }),
            ),
            // 4: adult rating.
            (
                4,
                json!({
                    "mal_id": 4, "rating": "Rx - Hentai", "type": "TV", "approved": true,
                    "genres": [], "explicit_genres": [], "themes": [], "demographics": []
                }),
            ),
            // 5: safe content but unapproved.
            (
                5,
                json!({
                    "mal_id": 5, "rating": "G - All Ages", "type": "TV", "approved": false,
                    "genres": [], "explicit_genres": [], "themes": [], "demographics": []
                }),
            ),
            // 6: erotica listed in demographics.
            (
                6,
                json!({
                    "mal_id": 6, "rating": "G - All Ages", "type": "TV", "approved": true,
                    "genres": [], "explicit_genres": [], "themes": [],
                    "demographics": [{"mal_id": 49, "name": "Erotica"}]
                }),
            ),
            // 8: missing rating/type fields pass the Mongo `$ne` checks.
            (
                8,
                json!({
                    "mal_id": 8, "approved": true,
                    "genres": [], "explicit_genres": [], "themes": [], "demographics": []
                }),
            ),
        ];
        for (mal_id, doc) in docs {
            store
                .upsert_entity(anime(mal_id, doc), None)
                .await
                .expect("upsert");
        }
        // A manga doujinshi is only excluded for manga queries.
        store
            .upsert_entity(
                StoredEntity::new(
                    EntityKind::Manga,
                    7,
                    json!({
                        "mal_id": 7, "rating": "G - All Ages", "type": "Doujinshi",
                        "approved": true, "genres": [], "explicit_genres": [],
                        "themes": [], "demographics": []
                    }),
                ),
                None,
            )
            .await
            .expect("upsert manga");

        let mut safe_ids: Vec<i64> = store
            .random_entities(EntityKind::Anime, 100, true, false)
            .await
            .expect("random")
            .iter()
            .map(|entity| entity.mal_id)
            .collect();
        safe_ids.sort_unstable();
        // PHP's scopeRandom only inspects `genres` and `demographics`, so
        // entries 2 (explicit_genres) and 3 (themes) are *not* excluded.
        // This quirk is intentional compatibility; see SFW_FILTER_SQL.
        assert_eq!(safe_ids, vec![1, 2, 3, 5, 8]);

        let unapproved_ids: Vec<i64> = store
            .random_entities(EntityKind::Anime, 100, true, true)
            .await
            .expect("random")
            .iter()
            .map(|entity| entity.mal_id)
            .collect();
        assert_eq!(unapproved_ids, vec![5]);

        assert_eq!(
            store
                .random_entities(EntityKind::Anime, 100, false, false)
                .await
                .expect("random")
                .len(),
            7,
            "without sfw every anime passes"
        );
        assert_eq!(
            store
                .random_entities(EntityKind::Anime, 1, true, false)
                .await
                .expect("random")
                .len(),
            1
        );
        assert!(store
            .random_entities(EntityKind::Anime, 0, true, false)
            .await
            .expect("random")
            .is_empty());
        assert!(store
            .random_entities(EntityKind::Manga, 10, true, false)
            .await
            .expect("random")
            .is_empty());
        assert_eq!(
            store
                .random_entities(EntityKind::Manga, 10, false, false)
                .await
                .expect("random")
                .len(),
            1
        );
    }

    #[test]
    fn entity_kind_string_round_trip() {
        for kind in EntityKind::ALL {
            assert_eq!(kind.to_string().parse::<EntityKind>().expect("parse"), kind);
            assert_eq!(
                serde_json::from_value::<EntityKind>(serde_json::json!(kind.as_str()))
                    .expect("deserialize"),
                kind
            );
        }
        assert!("nonsense".parse::<EntityKind>().is_err());
        assert_eq!(
            EntityKind::from_dump_key("characters"),
            Some(EntityKind::Character)
        );
        assert_eq!(
            EntityKind::from_dump_key("people"),
            Some(EntityKind::Person)
        );
        assert_eq!(EntityKind::from_dump_key("anime"), Some(EntityKind::Anime));
        assert_eq!(EntityKind::from_dump_key("nonsense"), None);
    }
}
