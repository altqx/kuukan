//! The workspace-wide entity vocabulary.
//!
//! [`EntityKind`] names the kinds of document kuukan stores, indexes and
//! serves. It lives here, rather than in `kuukan-store` or `kuukan-search`,
//! because both of those crates key their persistent state on it and the API
//! crate has to talk to both.
//!
//! Two different spellings of a kind reach disk, and both are load-bearing:
//!
//! * [`EntityKind::as_str`] is the value in the store's `kind` column, and the
//!   serde representation (`character`, `person`, `magazine`, ...). It is also
//!   what [`std::str::FromStr`] and [`std::fmt::Display`] speak.
//! * [`EntityKind::index_dir`] is the search sub-index directory name, which
//!   follows the jikan-rest table names instead (`characters`, `people`,
//!   `magazines`, ...). Only the searchable kinds have one.
//!
//! Neither spelling may change without migrating existing databases and
//! indexes, so they are kept apart deliberately rather than unified.

use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// A `kind` string that does not name an [`EntityKind`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidEntityKind(pub String);

impl std::fmt::Display for InvalidEntityKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown entity kind: {}", self.0)
    }
}

impl std::error::Error for InvalidEntityKind {}

/// The kind of an entity, matching the MongoDB collections of jikan-rest.
///
/// The serde representation is the snake_case string used in the store's
/// `kind` column: `anime`, `manga`, `character`, `person`, `user`, `club`,
/// `producer`, `magazine`, `genre_anime`, `genre_manga`, `episode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    /// `app/Anime.php`
    Anime,
    /// `app/Manga.php`
    Manga,
    /// `app/Character.php`
    Character,
    /// `app/Person.php`
    Person,
    /// `app/Profile.php`
    User,
    /// `app/Club.php`
    Club,
    /// `app/Producers.php`
    Producer,
    /// `app/Magazine.php`
    Magazine,
    GenreAnime,
    GenreManga,
    Episode,
}

impl EntityKind {
    /// Every kind, in declaration order.
    ///
    /// The searchable kinds come first, so [`EntityKind::SEARCHABLE`] is this
    /// list's prefix and both keep the same relative order.
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

    /// The kinds that carry a search sub-index, in a stable order.
    pub const SEARCHABLE: [EntityKind; 8] = [
        EntityKind::Anime,
        EntityKind::Manga,
        EntityKind::Character,
        EntityKind::Person,
        EntityKind::User,
        EntityKind::Club,
        EntityKind::Producer,
        EntityKind::Magazine,
    ];

    /// The canonical string value stored in the store's `kind` column.
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

    /// The search sub-index directory name, or `None` when the kind is not
    /// searchable.
    ///
    /// These follow the jikan-rest table names and are deliberately *not*
    /// [`EntityKind::as_str`]: they name directories that already exist on
    /// disk.
    pub const fn index_dir(self) -> Option<&'static str> {
        match self {
            EntityKind::Anime => Some("anime"),
            EntityKind::Manga => Some("manga"),
            EntityKind::Character => Some("characters"),
            EntityKind::Person => Some("people"),
            EntityKind::User => Some("users"),
            EntityKind::Club => Some("clubs"),
            EntityKind::Producer => Some("producers"),
            EntityKind::Magazine => Some("magazines"),
            EntityKind::GenreAnime | EntityKind::GenreManga | EntityKind::Episode => None,
        }
    }

    /// Whether this kind is fed into the search index.
    pub const fn is_searchable(self) -> bool {
        self.index_dir().is_some()
    }

    /// Resolve a search sub-index directory name back to its kind.
    pub fn from_index_dir(name: &str) -> Option<Self> {
        EntityKind::SEARCHABLE
            .into_iter()
            .find(|kind| kind.index_dir() == Some(name))
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
    type Err = InvalidEntityKind;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        EntityKind::ALL
            .into_iter()
            .find(|kind| kind.as_str() == value)
            .ok_or_else(|| InvalidEntityKind(value.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_str_round_trips_through_from_str() {
        for kind in EntityKind::ALL {
            assert_eq!(kind.as_str().parse::<EntityKind>(), Ok(kind));
            assert_eq!(kind.to_string(), kind.as_str());
        }
        assert!("nonsense".parse::<EntityKind>().is_err());
    }

    #[test]
    fn serde_uses_the_store_column_value() {
        for kind in EntityKind::ALL {
            let json = serde_json::to_value(kind).expect("serialize");
            assert_eq!(json, serde_json::json!(kind.as_str()));
            let back: EntityKind = serde_json::from_value(json).expect("deserialize");
            assert_eq!(back, kind);
        }
    }

    /// The store column values are persisted; pin them so a rename is a
    /// deliberate, visible change.
    #[test]
    fn store_column_values_are_pinned() {
        assert_eq!(EntityKind::Character.as_str(), "character");
        assert_eq!(EntityKind::Person.as_str(), "person");
        assert_eq!(EntityKind::User.as_str(), "user");
        assert_eq!(EntityKind::Magazine.as_str(), "magazine");
        assert_eq!(EntityKind::GenreAnime.as_str(), "genre_anime");
        assert_eq!(EntityKind::Episode.as_str(), "episode");
    }

    /// The index directories are on disk; they are plural and differ from the
    /// store column values for exactly six kinds.
    #[test]
    fn index_directories_are_pinned() {
        assert_eq!(EntityKind::Anime.index_dir(), Some("anime"));
        assert_eq!(EntityKind::Character.index_dir(), Some("characters"));
        assert_eq!(EntityKind::Person.index_dir(), Some("people"));
        assert_eq!(EntityKind::User.index_dir(), Some("users"));
        assert_eq!(EntityKind::Club.index_dir(), Some("clubs"));
        assert_eq!(EntityKind::Producer.index_dir(), Some("producers"));
        assert_eq!(EntityKind::Magazine.index_dir(), Some("magazines"));
        assert_eq!(EntityKind::GenreAnime.index_dir(), None);
        assert_eq!(EntityKind::Episode.index_dir(), None);
    }

    #[test]
    fn searchable_is_the_prefix_of_all() {
        assert_eq!(EntityKind::ALL[..8], EntityKind::SEARCHABLE);
        for kind in EntityKind::ALL {
            assert_eq!(kind.is_searchable(), EntityKind::SEARCHABLE.contains(&kind));
        }
    }

    #[test]
    fn index_dir_round_trips() {
        for kind in EntityKind::SEARCHABLE {
            let dir = kind.index_dir().expect("searchable kinds have a directory");
            assert_eq!(EntityKind::from_index_dir(dir), Some(kind));
        }
        assert_eq!(EntityKind::from_index_dir("episode"), None);
    }

    #[test]
    fn dump_keys_accept_the_mongo_collection_names() {
        assert_eq!(
            EntityKind::from_dump_key("characters"),
            Some(EntityKind::Character)
        );
        assert_eq!(
            EntityKind::from_dump_key("people"),
            Some(EntityKind::Person)
        );
        assert_eq!(
            EntityKind::from_dump_key("genres_anime"),
            Some(EntityKind::GenreAnime)
        );
        assert_eq!(
            EntityKind::from_dump_key("  ANIME "),
            Some(EntityKind::Anime)
        );
        assert_eq!(EntityKind::from_dump_key("nonsense"), None);
    }
}
