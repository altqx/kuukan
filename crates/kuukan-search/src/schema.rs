//! Tantivy schemas and payload extraction.
//!
//! One Tantivy index exists per [`EntityKind`]. The fields reproduce what each
//! Jikan model exposes to Typesense (`toSearchableArray`, `typesenseQueryBy`,
//! `getTypeSenseQueryByWeights`) plus the fields read by the PHP filters
//! (`app/Concerns/MediaFilters.php`, `app/Filters/*`).
//!
//! # Common fields (every kind)
//!
//! | Field         | Tantivy type            | Source (JMS payload)        |
//! |---------------|-------------------------|-----------------------------|
//! | `mal_id`      | u64 INDEXED FAST STORED | `mal_id`                    |
//! | `payload`     | STRING, stored only     | whole payload as JSON text  |
//! | `title_sort`  | raw text INDEXED FAST STORED, lowercased | display name (`title`, `name`, `username`, first producer title) |
//!
//! `title_sort` is the raw (untokenized) lowercase display name. It powers
//! `letter` prefix filters and `title`/`name` order-by, mirroring
//! `FilteredByLetter` and the sortable title field of the Typesense schema.
//!
//! # Full-text fields
//!
//! Weights are the `getTypeSenseQueryByWeights()` strings:
//! - anime/manga: `2,2,1,1,3,3,1` over
//!   `title, title_transformed, title_english, title_english_transformed,
//!    title_japanese, title_japanese_transformed, title_synonyms`
//! - character: `name, name_kanji` (no weights -> all 1)
//! - person: `name, given_name, family_name, alternate_names` (all 1)
//! - producer: `url, titles` (all 1)
//! - club/magazine/user: `name`/`username` (all 1)
//!
//! `*_transformed` is `simplifyStringForSearch`: every non-alphanumeric
//! character becomes a space.
//!
//! # Filter fields
//!
//! All filter fields are FAST, so they can also be used for `order_by`
//! (Tantivy `SortByErasedType`). Numeric/u64 list fields are additionally
//! INDEXED so term queries match any of their values.
//!
//! | Field(s) | Type | Source |
//! |---|---|---|
//! | `type`, `status`, `rating`, `source`, `season`, `broadcast_day` | raw string | literal payload fields (`season` derived from `premiered` when needed) |
//! | `score` | f64 | `score` |
//! | `scored_by`, `rank`, `popularity`, `members`, `favorites`, `episodes`, `year`, `chapters`, `volumes`, `count`, `member_favorites` | u64 | literal payload fields (`member_favorites` reads `favorites` first) |
//! | `start_date`, `end_date`, `birthday`, `established`, `created` | i64 unix seconds | `aired.from/to`, `published.from/to`, ISO-8601 strings; **omitted when null/absent** so range filters match Mongo semantics |
//! | `approved`, `airing` | bool | `approved ?? false`, `airing ?? false` |
//! | `genres`, `themes`, `demographics`, `explicit_genres`, `producers`, `studios`, `licensors`, `magazines` | u64 (multi) | list of `{mal_id}` objects; manga magazines read `magazines` or `serializations` |
//! | `category`, `access`, `gender`, `location` | raw string | club/profile fields (`access` falls back to `type`) |
//!
//! There is no `themes`/`demographics` in `Anime::toSearchableArray`, but the
//! Mongo filters (`MediaFilters::filterByGenres*`, sfw/kids scopes) do use
//! them, so they are indexed for anime and manga.

use chrono::{DateTime, NaiveDate, NaiveDateTime};
use serde_json::Value;
use tantivy::schema::{
    Field, IndexRecordOption, Schema, TantivyDocument, TextFieldIndexing, TextOptions, FAST,
    INDEXED, STORED, STRING, TEXT,
};

use crate::error::{Result, SearchError};

/// Name of the primary key field.
pub const MAL_ID: &str = "mal_id";
/// Name of the stored JSON payload field.
pub const PAYLOAD: &str = "payload";
/// Name of the lowercased display-name fast field.
pub const TITLE_SORT: &str = "title_sort";

/// Searchable entity kinds, one sub-index each.
///
/// `User` maps to the Jikan `Profile` model (`users` table); it is the kind
/// behind `GET /v1/users`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
}

impl EntityKind {
    /// Every kind, in a stable order.
    pub const ALL: [EntityKind; 8] = [
        EntityKind::Anime,
        EntityKind::Manga,
        EntityKind::Character,
        EntityKind::Person,
        EntityKind::User,
        EntityKind::Club,
        EntityKind::Producer,
        EntityKind::Magazine,
    ];

    /// Directory/collection name (mirrors the PHP table names).
    pub const fn as_str(self) -> &'static str {
        match self {
            EntityKind::Anime => "anime",
            EntityKind::Manga => "manga",
            EntityKind::Character => "characters",
            EntityKind::Person => "people",
            EntityKind::User => "users",
            EntityKind::Club => "clubs",
            EntityKind::Producer => "producers",
            EntityKind::Magazine => "magazines",
        }
    }

    /// Display-name attribute in the JMS payload.
    pub const fn title_attribute(self) -> &'static str {
        match self {
            EntityKind::Anime | EntityKind::Manga => "title",
            EntityKind::Character | EntityKind::Person => "name",
            EntityKind::User => "username",
            EntityKind::Club => "name",
            EntityKind::Producer => "titles",
            EntityKind::Magazine => "name",
        }
    }

    /// Full-text fields and their Typesense `query_by_weights`.
    pub const fn text_fields(self) -> &'static [(&'static str, f32)] {
        match self {
            EntityKind::Anime | EntityKind::Manga => &[
                ("title", 2.0),
                ("title_transformed", 2.0),
                ("title_english", 1.0),
                ("title_english_transformed", 1.0),
                ("title_japanese", 3.0),
                ("title_japanese_transformed", 3.0),
                ("title_synonyms", 1.0),
            ],
            EntityKind::Character => &[("name", 1.0), ("name_kanji", 1.0)],
            EntityKind::Person => &[
                ("name", 1.0),
                ("given_name", 1.0),
                ("family_name", 1.0),
                ("alternate_names", 1.0),
            ],
            EntityKind::User => &[("username", 1.0)],
            EntityKind::Club => &[("name", 1.0)],
            EntityKind::Producer => &[("url", 1.0), ("titles", 1.0)],
            EntityKind::Magazine => &[("name", 1.0)],
        }
    }

    /// Fast filter fields for this kind.
    pub const fn filter_fields(self) -> &'static [(&'static str, FieldType)] {
        match self {
            EntityKind::Anime => &[
                ("type", FieldType::Str),
                ("status", FieldType::Str),
                ("rating", FieldType::Str),
                ("source", FieldType::Str),
                ("season", FieldType::Str),
                ("broadcast_day", FieldType::Str),
                ("score", FieldType::F64),
                ("scored_by", FieldType::U64),
                ("rank", FieldType::U64),
                ("popularity", FieldType::U64),
                ("members", FieldType::U64),
                ("favorites", FieldType::U64),
                ("episodes", FieldType::U64),
                ("year", FieldType::U64),
                ("start_date", FieldType::I64),
                ("end_date", FieldType::I64),
                ("approved", FieldType::Bool),
                ("airing", FieldType::Bool),
                ("genres", FieldType::Ids),
                ("themes", FieldType::Ids),
                ("demographics", FieldType::Ids),
                ("explicit_genres", FieldType::Ids),
                ("producers", FieldType::Ids),
                ("studios", FieldType::Ids),
                ("licensors", FieldType::Ids),
            ],
            EntityKind::Manga => &[
                ("type", FieldType::Str),
                ("status", FieldType::Str),
                ("score", FieldType::F64),
                ("scored_by", FieldType::U64),
                ("rank", FieldType::U64),
                ("popularity", FieldType::U64),
                ("members", FieldType::U64),
                ("favorites", FieldType::U64),
                ("chapters", FieldType::U64),
                ("volumes", FieldType::U64),
                ("start_date", FieldType::I64),
                ("end_date", FieldType::I64),
                ("approved", FieldType::Bool),
                ("genres", FieldType::Ids),
                ("themes", FieldType::Ids),
                ("demographics", FieldType::Ids),
                ("explicit_genres", FieldType::Ids),
                ("magazines", FieldType::Ids),
            ],
            EntityKind::Character => &[
                ("member_favorites", FieldType::U64),
                ("start_date", FieldType::I64),
            ],
            EntityKind::Person => &[
                ("member_favorites", FieldType::U64),
                ("birthday", FieldType::I64),
            ],
            EntityKind::User => &[
                ("gender", FieldType::Str),
                ("location", FieldType::Str),
                ("birthday", FieldType::I64),
            ],
            EntityKind::Club => &[
                ("category", FieldType::Str),
                ("access", FieldType::Str),
                ("members", FieldType::U64),
                ("created", FieldType::I64),
            ],
            EntityKind::Producer => &[
                ("established", FieldType::I64),
                ("favorites", FieldType::U64),
                ("count", FieldType::U64),
            ],
            EntityKind::Magazine => &[("count", FieldType::U64)],
        }
    }

    /// Parse the on-disk/collection name.
    pub fn parse(name: &str) -> Option<EntityKind> {
        EntityKind::ALL.into_iter().find(|k| k.as_str() == name)
    }
}

impl std::fmt::Display for EntityKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for EntityKind {
    type Err = SearchError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        EntityKind::parse(s).ok_or_else(|| SearchError::UnknownKind(s.to_owned()))
    }
}

/// Physical field type of a filter field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldType {
    /// `u64`, indexed + fast.
    U64,
    /// `i64` (unix seconds), indexed + fast.
    I64,
    /// `f64`, indexed + fast.
    F64,
    /// `bool`, indexed + fast.
    Bool,
    /// Raw (untokenized) string, indexed + fast.
    Str,
    /// Multivalued `u64` (mal_id lists), indexed + fast.
    Ids,
}

/// Schema plus field handles for one entity kind.
pub struct EntitySchema {
    kind: EntityKind,
    schema: Schema,
    mal_id: Field,
    payload: Field,
    title_sort: Field,
    text_fields: Vec<Field>,
    text_boosts: Vec<f32>,
}

impl EntitySchema {
    /// Build the schema for `kind`.
    pub fn build(kind: EntityKind) -> Self {
        let mut builder = Schema::builder();

        let mal_id = builder.add_u64_field(MAL_ID, INDEXED | FAST | STORED);
        let payload = builder.add_text_field(PAYLOAD, TextOptions::default().set_stored());
        let title_sort = builder.add_text_field(TITLE_SORT, title_sort_options());

        let mut text_fields = Vec::new();
        let mut text_boosts = Vec::new();
        for (name, boost) in kind.text_fields() {
            let field = builder.add_text_field(name, TEXT);
            text_fields.push(field);
            text_boosts.push(*boost);
        }

        for (name, ty) in kind.filter_fields() {
            match ty {
                FieldType::U64 => {
                    builder.add_u64_field(name, INDEXED | FAST);
                }
                FieldType::I64 => {
                    builder.add_i64_field(name, INDEXED | FAST);
                }
                FieldType::F64 => {
                    builder.add_f64_field(name, INDEXED | FAST);
                }
                FieldType::Bool => {
                    builder.add_bool_field(name, INDEXED | FAST);
                }
                FieldType::Str => {
                    builder.add_text_field(name, STRING | FAST);
                }
                FieldType::Ids => {
                    builder.add_u64_field(name, INDEXED | FAST);
                }
            }
        }

        EntitySchema {
            kind,
            schema: builder.build(),
            mal_id,
            payload,
            title_sort,
            text_fields,
            text_boosts,
        }
    }

    /// The kind this schema describes.
    pub fn kind(&self) -> EntityKind {
        self.kind
    }

    /// Underlying Tantivy schema.
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Look up a field by name.
    pub fn field(&self, name: &str) -> Option<Field> {
        self.schema.get_field(name).ok()
    }

    /// Primary key field.
    pub fn mal_id_field(&self) -> Field {
        self.mal_id
    }

    /// Stored payload field.
    pub fn payload_field(&self) -> Field {
        self.payload
    }

    /// Lowercased display-name field.
    pub fn title_sort_field(&self) -> Field {
        self.title_sort
    }

    /// Full-text query fields, in `typesenseQueryBy()` order.
    pub fn text_fields(&self) -> &[Field] {
        &self.text_fields
    }

    /// Weights matching [`EntitySchema::text_fields`].
    pub fn text_boosts(&self) -> &[f32] {
        &self.text_boosts
    }

    /// Resolve an `order_by` parameter to an index field.
    ///
    /// Mirrors `SearchRequestHandler::prepareOrderByParam` (enum labels, e.g.
    /// `aired.from`/`published.from`) plus
    /// `TypeSenseScoutSearchService::overrideSortingOrder` (field names) and
    /// the enum label maps of `app/Enums/*OrderByEnum.php`.
    pub fn order_field(&self, order_by: &str) -> Option<Field> {
        let name = match (self.kind, order_by) {
            (EntityKind::Anime, "title") => TITLE_SORT,
            (EntityKind::Anime, "aired.from" | "start_date") => "start_date",
            (EntityKind::Anime, "aired.to" | "end_date") => "end_date",
            (EntityKind::Anime, "episodes") => "episodes",
            (
                EntityKind::Anime,
                "score" | "scored_by" | "rank" | "popularity" | "members" | "favorites" | "mal_id",
            ) => order_by,
            (EntityKind::Manga, "title") => TITLE_SORT,
            (EntityKind::Manga, "published.from" | "start_date") => "start_date",
            (EntityKind::Manga, "published.to" | "end_date") => "end_date",
            (
                EntityKind::Manga,
                "chapters" | "volumes" | "score" | "scored_by" | "rank" | "popularity" | "members"
                | "favorites" | "mal_id",
            ) => order_by,
            (EntityKind::Character, "name") => TITLE_SORT,
            (EntityKind::Character, "favorites" | "member_favorites" | "mal_id") => {
                if order_by == "mal_id" {
                    MAL_ID
                } else {
                    "member_favorites"
                }
            }
            (EntityKind::Person, "name") => TITLE_SORT,
            (EntityKind::Person, "favorites" | "member_favorites") => "member_favorites",
            (EntityKind::Person, "birthday" | "mal_id") => order_by,
            (EntityKind::User, "username" | "name") => TITLE_SORT,
            (EntityKind::User, "mal_id") => MAL_ID,
            (EntityKind::Club, "name") => TITLE_SORT,
            (EntityKind::Club, "members_count" | "members") => "members",
            (EntityKind::Club, "created" | "mal_id") => order_by,
            (EntityKind::Producer, "count" | "favorites" | "established" | "mal_id") => order_by,
            (EntityKind::Magazine, "name") => TITLE_SORT,
            (EntityKind::Magazine, "count" | "mal_id") => order_by,
            _ => return None,
        };
        self.field(name)
    }
}

/// Raw text options used for `title_sort`: untokenized, stored, fast.
fn title_sort_options() -> TextOptions {
    TextOptions::default()
        .set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer("raw")
                .set_index_option(IndexRecordOption::Basic),
        )
        .set_fast(Some("raw"))
        .set_stored()
}

/// Build a Tantivy document for `payload` according to `schema`.
///
/// The payload is the JMS-shaped JSON object (as served over HTTP), so this is
/// also what `SearchResult::items` returns verbatim.
pub fn build_document(schema: &EntitySchema, payload: &Value) -> Result<TantivyDocument> {
    if !payload.is_object() {
        return Err(SearchError::InvalidPayload(
            "expected a JSON object".to_owned(),
        ));
    }

    let mal_id = read_mal_id(payload)
        .ok_or_else(|| SearchError::InvalidPayload("missing mal_id".to_owned()))?;

    let mut doc = TantivyDocument::default();
    doc.add_u64(schema.mal_id, mal_id);
    doc.add_text(schema.payload, serde_json::to_string(payload)?);

    let display_name = display_name(schema.kind(), payload).unwrap_or_default();
    doc.add_text(schema.title_sort, display_name.to_lowercase());

    match schema.kind() {
        EntityKind::Anime => fill_anime(schema, &mut doc, payload, &display_name),
        EntityKind::Manga => fill_manga(schema, &mut doc, payload),
        EntityKind::Character => fill_character(schema, &mut doc, payload),
        EntityKind::Person => fill_person(schema, &mut doc, payload),
        EntityKind::User => fill_user(schema, &mut doc, payload),
        EntityKind::Club => fill_club(schema, &mut doc, payload),
        EntityKind::Producer => fill_producer(schema, &mut doc, payload),
        EntityKind::Magazine => fill_magazine(schema, &mut doc, payload),
    }

    Ok(doc)
}

/// Read `mal_id` from a payload (number or numeric string).
pub fn read_mal_id(payload: &Value) -> Option<u64> {
    get(payload, "mal_id").and_then(as_u64)
}

/// Display name used for `title_sort` and the letter filter.
pub fn display_name(kind: EntityKind, payload: &Value) -> Option<String> {
    match kind {
        EntityKind::Producer => payload
            .get("titles")
            .and_then(Value::as_array)
            .and_then(|titles| titles.first())
            .and_then(|t| get(t, "title"))
            .and_then(as_string),
        _ => get(payload, kind.title_attribute()).and_then(as_string),
    }
}

/// Port of `JikanApiSearchableModel::simplifyStringForSearch`.
pub fn simplify_string(value: &str) -> String {
    value
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect()
}

// ---------------------------------------------------------------------------
// per-kind extraction
// ---------------------------------------------------------------------------

fn fill_anime(
    schema: &EntitySchema,
    doc: &mut TantivyDocument,
    payload: &Value,
    display_name: &str,
) {
    add_title_fields(schema, doc, payload, Some(display_name));

    copy_str(schema, doc, payload, "type");
    copy_str(schema, doc, payload, "status");
    copy_str(schema, doc, payload, "rating");
    copy_str(schema, doc, payload, "source");
    copy_u64(schema, doc, payload, "scored_by");
    copy_u64(schema, doc, payload, "rank");
    copy_u64(schema, doc, payload, "popularity");
    copy_u64(schema, doc, payload, "members");
    copy_u64(schema, doc, payload, "favorites");
    copy_u64(schema, doc, payload, "episodes");
    copy_u64(schema, doc, payload, "year");
    copy_f64(schema, doc, payload, "score");
    copy_timestamp(schema, doc, payload, "start_date", "aired.from");
    copy_timestamp(schema, doc, payload, "end_date", "aired.to");
    copy_bool(schema, doc, payload, "approved", false);
    copy_bool(schema, doc, payload, "airing", false);

    // `season`/`year` are appended accessors derived from `premiered`.
    if get(payload, "season").and_then(as_string).is_none() {
        if let Some(premiered) = get(payload, "premiered").and_then(as_string) {
            if let Some(season) = premiered.split_whitespace().next() {
                add_str(schema, doc, "season", season);
            }
        }
    }
    if get(payload, "year").and_then(as_u64).is_none() {
        if let Some(premiered) = get(payload, "premiered").and_then(as_string) {
            if let Some(year) = premiered
                .split_whitespace()
                .nth(1)
                .and_then(|y| y.parse().ok())
            {
                add_u64(schema, doc, "year", year);
            }
        }
    }
    if let Some(day) = get(payload, "broadcast.day").and_then(as_string) {
        add_str(schema, doc, "broadcast_day", &day);
    }

    add_ids(schema, doc, payload, "genres");
    add_ids(schema, doc, payload, "themes");
    add_ids(schema, doc, payload, "demographics");
    add_ids(schema, doc, payload, "explicit_genres");
    add_ids(schema, doc, payload, "producers");
    add_ids(schema, doc, payload, "studios");
    add_ids(schema, doc, payload, "licensors");
}

fn fill_manga(schema: &EntitySchema, doc: &mut TantivyDocument, payload: &Value) {
    add_title_fields(schema, doc, payload, None);

    copy_str(schema, doc, payload, "type");
    copy_str(schema, doc, payload, "status");
    copy_u64(schema, doc, payload, "scored_by");
    copy_u64(schema, doc, payload, "rank");
    copy_u64(schema, doc, payload, "popularity");
    copy_u64(schema, doc, payload, "members");
    copy_u64(schema, doc, payload, "favorites");
    copy_u64(schema, doc, payload, "chapters");
    copy_u64(schema, doc, payload, "volumes");
    copy_f64(schema, doc, payload, "score");
    copy_timestamp(schema, doc, payload, "start_date", "published.from");
    copy_timestamp(schema, doc, payload, "end_date", "published.to");
    copy_bool(schema, doc, payload, "approved", false);

    add_ids(schema, doc, payload, "genres");
    add_ids(schema, doc, payload, "themes");
    add_ids(schema, doc, payload, "demographics");
    add_ids(schema, doc, payload, "explicit_genres");
    // The Mongo model calls this `serializations`; the Typesense document and
    // the `magazines` filter use `magazines`.
    if get(payload, "magazines").is_some() {
        add_ids(schema, doc, payload, "magazines");
    } else {
        add_ids_from(schema, doc, payload, "magazines", "serializations");
    }
}

fn fill_character(schema: &EntitySchema, doc: &mut TantivyDocument, payload: &Value) {
    add_text(
        schema,
        doc,
        "name",
        get(payload, "name").and_then(as_string),
    );
    add_text(
        schema,
        doc,
        "name_kanji",
        get(payload, "name_kanji").and_then(as_string),
    );
    copy_u64_first(
        schema,
        doc,
        payload,
        "member_favorites",
        &["favorites", "member_favorites"],
    );
}

fn fill_person(schema: &EntitySchema, doc: &mut TantivyDocument, payload: &Value) {
    add_text(
        schema,
        doc,
        "name",
        get(payload, "name").and_then(as_string),
    );
    add_text(
        schema,
        doc,
        "given_name",
        get(payload, "given_name").and_then(as_string),
    );
    add_text(
        schema,
        doc,
        "family_name",
        get(payload, "family_name").and_then(as_string),
    );
    add_text_list(
        schema,
        doc,
        "alternate_names",
        get(payload, "alternate_names"),
    );
    copy_u64_first(
        schema,
        doc,
        payload,
        "member_favorites",
        &["favorites", "member_favorites"],
    );
    copy_timestamp(schema, doc, payload, "birthday", "birthday");
}

fn fill_user(schema: &EntitySchema, doc: &mut TantivyDocument, payload: &Value) {
    add_text(
        schema,
        doc,
        "username",
        get(payload, "username").and_then(as_string),
    );
    copy_str(schema, doc, payload, "gender");
    copy_str(schema, doc, payload, "location");
    copy_timestamp(schema, doc, payload, "birthday", "birthday");
}

fn fill_club(schema: &EntitySchema, doc: &mut TantivyDocument, payload: &Value) {
    add_text(
        schema,
        doc,
        "name",
        get(payload, "name").and_then(as_string),
    );
    copy_str(schema, doc, payload, "category");
    if get(payload, "access").is_some() {
        copy_str(schema, doc, payload, "access");
    } else {
        copy_str_from(schema, doc, payload, "access", "type");
    }
    copy_u64(schema, doc, payload, "members");
    copy_timestamp(schema, doc, payload, "created", "created");
}

fn fill_producer(schema: &EntitySchema, doc: &mut TantivyDocument, payload: &Value) {
    add_text(schema, doc, "url", get(payload, "url").and_then(as_string));
    if let Some(titles) = get(payload, "titles").and_then(Value::as_array) {
        for title in titles {
            add_text(
                schema,
                doc,
                "titles",
                get(title, "title").and_then(as_string),
            );
        }
    }
    copy_u64(schema, doc, payload, "favorites");
    copy_u64(schema, doc, payload, "count");
    copy_timestamp(schema, doc, payload, "established", "established");
}

fn fill_magazine(schema: &EntitySchema, doc: &mut TantivyDocument, payload: &Value) {
    add_text(
        schema,
        doc,
        "name",
        get(payload, "name").and_then(as_string),
    );
    copy_u64(schema, doc, payload, "count");
}

/// Anime/manga title fields. Weights come from `getTypeSenseQueryByWeights`.
fn add_title_fields(
    schema: &EntitySchema,
    doc: &mut TantivyDocument,
    payload: &Value,
    display_name: Option<&str>,
) {
    let title = get(payload, "title").and_then(as_string);
    add_text(schema, doc, "title", title.clone());

    // `title_transformed` is `simplifyStringForSearch($this->title)`.
    let transformed = match &title {
        Some(t) => Some(simplify_string(t)),
        None => display_name.map(simplify_string),
    };
    add_text(schema, doc, "title_transformed", transformed);

    for (raw, transformed_field) in [
        ("title_english", "title_english_transformed"),
        ("title_japanese", "title_japanese_transformed"),
    ] {
        let value = get(payload, raw)
            .and_then(as_string)
            .filter(|s| !s.is_empty());
        add_text(schema, doc, raw, value.clone());
        add_text(
            schema,
            doc,
            transformed_field,
            value.as_deref().map(simplify_string),
        );
    }

    // `title_synonyms` = titles whose type is not Default/English/Japanese.
    let mut synonyms: Vec<String> = get(payload, "titles")
        .and_then(Value::as_array)
        .map(|titles| {
            titles
                .iter()
                .filter(|t| {
                    let kind = get(t, "type").and_then(as_string).unwrap_or_default();
                    !matches!(kind.as_str(), "Default" | "English" | "Japanese")
                })
                .filter_map(|t| get(t, "title").and_then(as_string))
                .collect()
        })
        .unwrap_or_default();
    if synonyms.is_empty() {
        synonyms = get(payload, "title_synonyms")
            .and_then(Value::as_array)
            .map(|values| values.iter().filter_map(as_string).collect())
            .unwrap_or_default();
    }
    for synonym in synonyms {
        add_text(schema, doc, "title_synonyms", Some(synonym));
    }
}

// ---------------------------------------------------------------------------
// small JSON/extraction helpers
// ---------------------------------------------------------------------------

/// Read a dotted path from a JSON value. `null` counts as absent.
pub fn get<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for part in path.split('.') {
        current = current.get(part)?;
    }
    if current.is_null() {
        None
    } else {
        Some(current)
    }
}

/// Number or numeric string to `u64`.
pub fn as_u64(value: &Value) -> Option<u64> {
    match value {
        Value::Number(n) => n.as_u64().or_else(|| n.as_f64().map(|f| f as u64)),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// Number or numeric string to `i64`.
pub fn as_i64(value: &Value) -> Option<i64> {
    match value {
        Value::Number(n) => n.as_i64().or_else(|| n.as_f64().map(|f| f as i64)),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// Number, numeric string or string to `f64`.
pub fn as_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// Boolean or boolean-ish string.
pub fn as_bool(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(b) => Some(*b),
        Value::String(s) => match s.trim().to_ascii_lowercase().as_str() {
            "true" | "1" => Some(true),
            "false" | "0" => Some(false),
            _ => None,
        },
        Value::Number(n) => n.as_i64().map(|n| n != 0),
        _ => None,
    }
}

/// String value.
pub fn as_string(value: &Value) -> Option<String> {
    value.as_str().map(str::to_owned)
}

/// Parse an ISO-8601/`DATE_ATOM`/`YYYY-MM-DD` date into unix seconds.
///
/// Numbers are treated as already-unix timestamps.
pub fn parse_timestamp(value: &Value) -> Option<i64> {
    match value {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => parse_timestamp_str(s),
        _ => None,
    }
}

fn parse_timestamp_str(value: &str) -> Option<i64> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if let Ok(datetime) = DateTime::parse_from_rfc3339(value) {
        return Some(datetime.timestamp());
    }
    for format in [
        "%Y-%m-%dT%H:%M:%S%.f%z",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
    ] {
        if let Ok(datetime) = NaiveDateTime::parse_from_str(value, format) {
            return Some(datetime.and_utc().timestamp());
        }
    }
    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        return date.and_hms_opt(0, 0, 0).map(|dt| dt.and_utc().timestamp());
    }
    None
}

fn add_text(schema: &EntitySchema, doc: &mut TantivyDocument, field: &str, value: Option<String>) {
    if let (Some(field), Some(value)) = (schema.field(field), value) {
        if !value.is_empty() {
            doc.add_text(field, value);
        }
    }
}

fn add_text_list(
    schema: &EntitySchema,
    doc: &mut TantivyDocument,
    field: &str,
    value: Option<&Value>,
) {
    let Some(field) = schema.field(field) else {
        return;
    };
    match value {
        Some(Value::Array(values)) => {
            for value in values {
                if let Some(text) = as_string(value).filter(|s| !s.is_empty()) {
                    doc.add_text(field, text);
                }
            }
        }
        Some(other) => {
            if let Some(text) = as_string(other).filter(|s| !s.is_empty()) {
                doc.add_text(field, text);
            }
        }
        None => {}
    }
}

fn add_str(schema: &EntitySchema, doc: &mut TantivyDocument, field: &str, value: &str) {
    if let Some(field) = schema.field(field) {
        doc.add_text(field, value);
    }
}

fn add_u64(schema: &EntitySchema, doc: &mut TantivyDocument, field: &str, value: u64) {
    if let Some(field) = schema.field(field) {
        doc.add_u64(field, value);
    }
}

fn add_i64(schema: &EntitySchema, doc: &mut TantivyDocument, field: &str, value: i64) {
    if let Some(field) = schema.field(field) {
        doc.add_i64(field, value);
    }
}

fn add_f64(schema: &EntitySchema, doc: &mut TantivyDocument, field: &str, value: f64) {
    if let Some(field) = schema.field(field) {
        doc.add_f64(field, value);
    }
}

fn add_bool(schema: &EntitySchema, doc: &mut TantivyDocument, field: &str, value: bool) {
    if let Some(field) = schema.field(field) {
        doc.add_bool(field, value);
    }
}

fn copy_str(schema: &EntitySchema, doc: &mut TantivyDocument, payload: &Value, field: &str) {
    if let Some(value) = get(payload, field).and_then(as_string) {
        add_str(schema, doc, field, &value);
    }
}

fn copy_str_from(
    schema: &EntitySchema,
    doc: &mut TantivyDocument,
    payload: &Value,
    field: &str,
    source: &str,
) {
    if let Some(value) = get(payload, source).and_then(as_string) {
        add_str(schema, doc, field, &value);
    }
}

fn copy_u64(schema: &EntitySchema, doc: &mut TantivyDocument, payload: &Value, field: &str) {
    if let Some(value) = get(payload, field).and_then(as_u64) {
        add_u64(schema, doc, field, value);
    }
}

/// Like [`copy_u64`] but tries several payload keys in order (e.g.
/// `favorites` then `member_favorites`, matching the resource accessors).
fn copy_u64_first(
    schema: &EntitySchema,
    doc: &mut TantivyDocument,
    payload: &Value,
    field: &str,
    sources: &[&str],
) {
    for source in sources {
        if let Some(value) = get(payload, source).and_then(as_u64) {
            add_u64(schema, doc, field, value);
            return;
        }
    }
}

fn copy_f64(schema: &EntitySchema, doc: &mut TantivyDocument, payload: &Value, field: &str) {
    if let Some(value) = get(payload, field).and_then(as_f64) {
        add_f64(schema, doc, field, value);
    }
}

fn copy_bool(
    schema: &EntitySchema,
    doc: &mut TantivyDocument,
    payload: &Value,
    field: &str,
    default: bool,
) {
    let value = get(payload, field).and_then(as_bool).unwrap_or(default);
    add_bool(schema, doc, field, value);
}

fn copy_timestamp(
    schema: &EntitySchema,
    doc: &mut TantivyDocument,
    payload: &Value,
    field: &str,
    source: &str,
) {
    if let Some(value) = get(payload, source).and_then(parse_timestamp) {
        add_i64(schema, doc, field, value);
    }
}

/// Add `mal_id`s from an array of `{mal_id}` objects or plain numbers.
fn add_ids(schema: &EntitySchema, doc: &mut TantivyDocument, payload: &Value, field: &str) {
    add_ids_from(schema, doc, payload, field, field);
}

/// Add `mal_id`s from `source` into the index field `field`.
fn add_ids_from(
    schema: &EntitySchema,
    doc: &mut TantivyDocument,
    payload: &Value,
    field: &str,
    source: &str,
) {
    let Some(field_handle) = schema.field(field) else {
        return;
    };
    let Some(Value::Array(values)) = get(payload, source) else {
        return;
    };
    for value in values {
        let id = match value {
            Value::Object(_) => get(value, "mal_id").and_then(as_u64),
            other => as_u64(other),
        };
        if let Some(id) = id {
            doc.add_u64(field_handle, id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tantivy::schema::Value as TantivyValue;

    #[test]
    fn every_kind_builds_a_schema() {
        for kind in EntityKind::ALL {
            let schema = EntitySchema::build(kind);
            assert!(schema.field(MAL_ID).is_some());
            assert!(schema.field(PAYLOAD).is_some());
            assert!(schema.field(TITLE_SORT).is_some());
            for (name, _) in kind.filter_fields() {
                assert!(schema.field(name).is_some(), "{kind}: {name}");
            }
            assert_eq!(schema.text_fields().len(), kind.text_fields().len());
        }
    }

    #[test]
    fn kind_names_roundtrip() {
        for kind in EntityKind::ALL {
            assert_eq!(EntityKind::parse(kind.as_str()), Some(kind));
        }
    }

    #[test]
    fn simplify_string_replaces_punctuation() {
        assert_eq!(simplify_string("Re:Zero"), "Re Zero");
        assert_eq!(
            simplify_string("Boku no Hero Academia 2nd Season"),
            "Boku no Hero Academia 2nd Season"
        );
    }

    #[test]
    fn parses_dates() {
        assert_eq!(
            parse_timestamp(&json!("1998-04-03T00:00:00+00:00")),
            Some(891561600)
        );
        assert_eq!(parse_timestamp(&json!("1998-04-03")), Some(891561600));
        assert_eq!(parse_timestamp(&json!(1234567890)), Some(1234567890));
        assert_eq!(parse_timestamp(&json!(null)), None);
        assert_eq!(parse_timestamp(&json!("")), None);
    }

    #[test]
    fn builds_an_anime_document() {
        let schema = EntitySchema::build(EntityKind::Anime);
        let payload = json!({
            "mal_id": 1,
            "title": "Cowboy Bebop",
            "title_english": "Cowboy Bebop",
            "title_japanese": "カウボーイビバップ",
            "titles": [
                {"type": "Default", "title": "Cowboy Bebop"},
                {"type": "Synonym", "title": "Cowboy Bebop: The Movie"}
            ],
            "type": "TV",
            "status": "Finished Airing",
            "rating": "R - 17+ (violence & profanity)",
            "score": 8.75,
            "rank": 40,
            "popularity": 45,
            "members": 1800000,
            "favorites": 100000,
            "aired": {"from": "1998-04-03T00:00:00+00:00", "to": "1999-04-24T00:00:00+00:00"},
            "approved": true,
            "airing": false,
            "season": "spring",
            "year": 1998,
            "broadcast": {"day": "Saturdays"},
            "genres": [{"mal_id": 1}, {"mal_id": 24}],
            "themes": [{"mal_id": 50}],
            "demographics": [{"mal_id": 27}],
            "studios": [{"mal_id": 14}],
        });
        let doc = build_document(&schema, &payload).unwrap();

        let stored: serde_json::Value = serde_json::from_str(
            doc.get_first(schema.payload_field())
                .unwrap()
                .as_str()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(stored["mal_id"], 1);
        assert_eq!(
            doc.get_first(schema.title_sort_field()).unwrap().as_str(),
            Some("cowboy bebop")
        );
        assert_eq!(
            doc.get_first(schema.field("start_date").unwrap())
                .unwrap()
                .as_i64(),
            Some(891561600)
        );
        assert_eq!(
            doc.get_first(schema.field("approved").unwrap())
                .unwrap()
                .as_bool(),
            Some(true)
        );
    }

    #[test]
    fn null_dates_are_omitted() {
        let schema = EntitySchema::build(EntityKind::Anime);
        let payload = json!({
            "mal_id": 1,
            "title": "Upcoming",
            "aired": {"from": null, "to": null},
        });
        let doc = build_document(&schema, &payload).unwrap();
        assert!(doc.get_first(schema.field("start_date").unwrap()).is_none());
        assert!(doc.get_first(schema.field("end_date").unwrap()).is_none());
        assert_eq!(
            doc.get_first(schema.field("approved").unwrap())
                .unwrap()
                .as_bool(),
            Some(false)
        );
    }
}
