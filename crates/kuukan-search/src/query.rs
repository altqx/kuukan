//! Search parameter semantics: a port of the Jikan Typesense pipeline
//! (`TypeSenseScoutSearchService` + `DefaultQueryBuilderService`) on Tantivy.
//!
//! # Ranking divergence
//!
//! Typesense's `_text_match` score is proprietary; Tantivy uses BM25. The
//! ranking *shape* is preserved (`_text_match:desc` is the default when `q` is
//! present, tie-breakers keep their PHP order) but exact hit ordering on ties
//! may differ; ties may be ordered differently.
//!
//! # Short queries (`adaptToShortQueries`)
//!
//! `q.len() <= 3` disables fuzziness, searches terms as prefixes, and adapts
//! the sort key:
//! - no `order_by`: `_text_match:desc, <title>:asc`
//! - with `order_by`: `_text_match:desc, <order_by>:<dir>`
//!
//! # Filters
//!
//! - `genres`/`genres_exclude`: AND across comma-separated ids, OR across
//!   `genres|themes|demographics|explicit_genres` (`MediaFilters`).
//! - `producers`: OR across `producers|licensors|studios`.
//! - `min_score == 0` / `max_score == 10` are ignored (issue #309).
//! - `sfw` uses `scopeExceptItemsWithAdultRating`, `kids` the kids scopes,
//!   `unapproved` defaults to `approved = true`.
//! - date filters are inclusive (`>=` start, `<=` end) at 00:00 UTC, matching
//!   `filterByStartDate`/`filterByEndDate`.

use std::ops::Bound;

use chrono::NaiveDate;
use serde::Serialize;
use serde_json::Value;
use tantivy::collector::sort_key::{ComparatorEnum, SortByErasedType};
use tantivy::collector::{Count, TopDocs};
use tantivy::query::{
    AllQuery, BooleanQuery, BoostQuery, EmptyQuery, Occur, PhrasePrefixQuery, Query, QueryParser,
    RangeQuery, TermQuery,
};
use tantivy::schema::Value as TantivyValue;
use tantivy::schema::{Field, IndexRecordOption, TantivyDocument, Term};
use tantivy::{DocAddress, Index, Score, Searcher};

use crate::error::Result;
use crate::index::SearchIndex;
use crate::schema::{EntityKind, EntitySchema};

/// Default page size when `MAX_RESULTS_PER_PAGE` is unset (PHP default).
pub const DEFAULT_PER_PAGE: u64 = 25;

/// Hard ceiling applied by the Typesense middleware (`per_page > 250`).
pub const MAX_PER_PAGE_CEILING: u64 = 250;

/// Environment variable honoured for the page-size default/cap.
pub const MAX_RESULTS_PER_PAGE_ENV: &str = "MAX_RESULTS_PER_PAGE";

/// `MAX_RESULTS_PER_PAGE`, capped the way the Typesense middleware caps it.
///
/// The PHP DTO rejects `limit > MAX_RESULTS_PER_PAGE`; this crate clamps
/// instead (validation belongs to the HTTP layer).
pub fn max_per_page() -> u64 {
    std::env::var(MAX_RESULTS_PER_PAGE_ENV)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_PER_PAGE)
        .min(MAX_PER_PAGE_CEILING)
}

/// Sort direction for `sort=asc|desc`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortDirection {
    /// `asc`.
    Asc,
    /// `desc`.
    Desc,
}

impl SortDirection {
    /// Parse an `asc`/`desc` query value (case-insensitive).
    pub fn parse(value: &str) -> Option<SortDirection> {
        match value.trim().to_ascii_lowercase().as_str() {
            "asc" => Some(SortDirection::Asc),
            "desc" => Some(SortDirection::Desc),
            _ => None,
        }
    }

    /// Whether this direction is descending.
    pub fn is_desc(self) -> bool {
        matches!(self, SortDirection::Desc)
    }
}

/// Everything a Jikan search endpoint accepts.
///
/// Values are the *parsed* ones the HTTP layer already validated against the
/// `app/Dto/*SearchCommand` enums: e.g. `media_type = "TV"` (the enum label)
/// and `status = "Finished Airing"`, not the query-string enum keys.
#[derive(Clone, Debug, Default)]
pub struct SearchParams {
    /// `q` — free text search terms.
    pub q: Option<String>,
    /// `page` — 1-based page number.
    pub page: Option<u64>,
    /// `limit` — page size.
    pub limit: Option<u64>,
    /// `order_by` — parsed enum label/field (`title`, `aired.from`, ...).
    pub order_by: Option<String>,
    /// `sort` — `asc`/`desc`.
    pub sort: Option<SortDirection>,
    /// `letter` — display-name prefix filter (prohibits `q`).
    pub letter: Option<String>,
    /// `type` — media/club type label (`"TV"`, `"Manga"`, ...).
    pub media_type: Option<String>,
    /// `status` — status label (`"Finished Airing"`, ...).
    pub status: Option<String>,
    /// `rating` — anime rating label (`"Rx - Hentai"`, ...).
    pub rating: Option<String>,
    /// `score` — exact score.
    pub score: Option<f64>,
    /// `min_score` — inclusive minimum score (0 ignored).
    pub min_score: Option<f64>,
    /// `max_score` — inclusive maximum score (10 ignored).
    pub max_score: Option<f64>,
    /// `start_date` — `YYYY-MM-DD`, inclusive lower bound on the start date.
    pub start_date: Option<String>,
    /// `end_date` — `YYYY-MM-DD`, inclusive upper bound on the end date.
    pub end_date: Option<String>,
    /// `producer` — single producer mal_id (producers/licensors/studios).
    pub producer: Option<u64>,
    /// `producers` — comma-separated producer mal_ids.
    pub producers: Option<String>,
    /// `magazine` — single magazine mal_id.
    pub magazine: Option<u64>,
    /// `magazines` — comma-separated magazine mal_ids.
    pub magazines: Option<String>,
    /// `genres` — comma-separated genre mal_ids (all required).
    pub genres: Option<String>,
    /// `genres_exclude` — comma-separated genre mal_ids (all excluded).
    pub genres_exclude: Option<String>,
    /// `sfw` — exclude adult ratings/genres.
    pub sfw: Option<bool>,
    /// `unapproved` — when not `true`, only approved entries are returned.
    pub unapproved: Option<bool>,
    /// `kids` — `true`: only kids entries, `false`: exclude them.
    pub kids: Option<bool>,
    /// `category` — club category label.
    pub category: Option<String>,
    /// `minAge` — reserved for user search (scraped by Jikan, unused here).
    pub min_age: Option<u64>,
    /// `maxAge` — reserved for user search.
    pub max_age: Option<u64>,
    /// `gender` — user gender filter.
    pub gender: Option<String>,
    /// `location` — user location filter.
    pub location: Option<String>,
}

/// Paginated search result.
///
/// Shapes the `ResultsResource` (`total`, `current_page`, `per_page`,
/// `last_page`) plus the hydrated payloads.
#[derive(Clone, Debug, Serialize)]
pub struct SearchResult {
    /// Matching payloads for the requested page.
    pub items: Vec<Value>,
    /// Total matching documents.
    pub total: u64,
    /// Current page (1-based).
    pub current_page: u64,
    /// Effective page size.
    pub per_page: u64,
    /// Last page, `max(1, ceil(total / per_page))`.
    pub last_page: u64,
}

impl SearchResult {
    /// Empty result for a page.
    pub fn empty(page: u64, per_page: u64) -> SearchResult {
        SearchResult {
            items: Vec::new(),
            total: 0,
            current_page: page,
            per_page,
            last_page: 1,
        }
    }
}

/// A resolved sort key.
#[derive(Clone, Debug, PartialEq)]
enum SortKeySpec {
    /// `_text_match` / BM25 relevance.
    Score,
    /// Fast-field sort.
    Field {
        field: Field,
        direction: SortDirection,
    },
}

impl SearchIndex {
    /// Run a search against the index of `kind`.
    pub fn search(&self, kind: EntityKind, params: &SearchParams) -> Result<SearchResult> {
        let schema = self.schema(kind)?;
        let index = self.index(kind)?;
        let searcher = self.searcher(kind)?;

        let page = params.page.unwrap_or(1).max(1);
        let per_page = params
            .limit
            .unwrap_or_else(max_per_page)
            .clamp(1, max_per_page());

        let has_text = ends_up_as_text_search(params);
        let query = build_query(schema, index, params, has_text)?;

        let total = searcher.search(query.as_ref(), &Count)? as u64;
        if total == 0 {
            return Ok(SearchResult::empty(page, per_page));
        }

        let last_page = total.div_ceil(per_page).max(1);
        let offset = page.saturating_sub(1).saturating_mul(per_page);
        let limit = per_page as usize;
        let offset = offset as usize;

        let sort_keys = sort_plan(schema, params, has_text);
        let addresses = collect_top_docs(&searcher, query.as_ref(), &sort_keys, limit, offset)?;
        let items = fetch_payloads(&searcher, schema, &addresses)?;

        Ok(SearchResult {
            items,
            total,
            current_page: page,
            per_page,
            last_page,
        })
    }
}

/// `q` present and no `letter` (the `DefaultQueryBuilderService` split).
fn ends_up_as_text_search(params: &SearchParams) -> bool {
    let has_q = params
        .q
        .as_deref()
        .map(str::trim)
        .is_some_and(|q| !q.is_empty());
    let has_letter = params
        .letter
        .as_deref()
        .map(str::trim)
        .is_some_and(|l| !l.is_empty());
    has_q && !has_letter
}

/// `strlen($query) <= 3` of `TypeSenseScoutSearchService::adaptToShortQueries`
/// (PHP `strlen` counts bytes, so Rust `str::len` is the faithful port).
fn is_short_query(params: &SearchParams) -> bool {
    params
        .q
        .as_deref()
        .map(str::trim)
        .is_some_and(|q| q.len() <= 3)
}

/// Build the full boolean query: text/letter base + all filter clauses.
fn build_query(
    schema: &EntitySchema,
    index: &Index,
    params: &SearchParams,
    has_text: bool,
) -> Result<Box<dyn Query>> {
    let base: Box<dyn Query> = if has_text {
        text_query(schema, index, params.q.as_deref().unwrap_or_default())
    } else if let Some(letter) = params
        .letter
        .as_deref()
        .map(str::trim)
        .filter(|l| !l.is_empty())
    {
        letter_query(schema, letter)
    } else {
        Box::new(AllQuery)
    };

    let mut clauses: Vec<(Occur, Box<dyn Query>)> = vec![(Occur::Must, base)];
    add_filters(schema, params, &mut clauses);
    Ok(Box::new(BooleanQuery::new(clauses)))
}

/// Text query over the kind's title fields with the PHP short-query adaptation.
fn text_query(schema: &EntitySchema, index: &Index, q: &str) -> Box<dyn Query> {
    let q = q.trim();
    if q.is_empty() {
        return Box::new(AllQuery);
    }

    let mut parser = QueryParser::for_index(index, schema.text_fields().to_vec());
    for (field, boost) in schema.text_fields().iter().zip(schema.text_boosts()) {
        parser.set_field_boost(*field, *boost as Score);
    }

    let short = q.len() <= 3;
    if !short {
        for field in schema.text_fields() {
            parser.set_field_fuzzy(*field, false, 1, true);
        }
        let (query, _errors) = parser.parse_query_lenient(q);
        return query;
    }

    // Short query: no typo tolerance, prefix matching on the last token and an
    // exact/prefix boost (`prioritize_token_position: true` in Typesense
    // terms). Tantivy's query parser ignores a bare `term*`, so the per-field
    // `PhrasePrefixQuery` is built from the field tokenizer directly.
    short_query(schema, index, q)
}

/// Prefix query for `q.len() <= 3`: every token exact, the last one a prefix,
/// OR-ed across title fields with their Typesense weights.
fn short_query(schema: &EntitySchema, index: &Index, q: &str) -> Box<dyn Query> {
    let mut should: Vec<(Occur, Box<dyn Query>)> = Vec::new();
    for (field, boost) in schema.text_fields().iter().zip(schema.text_boosts()) {
        let Ok(mut analyzer) = index.tokenizer_for_field(*field) else {
            continue;
        };
        let mut terms: Vec<Term> = Vec::new();
        analyzer.token_stream(q).process(&mut |token| {
            terms.push(Term::from_field_text(*field, &token.text));
        });
        if terms.is_empty() {
            continue;
        }
        let query: Box<dyn Query> = Box::new(PhrasePrefixQuery::new(terms));
        let query: Box<dyn Query> = if (*boost - 1.0).abs() > f32::EPSILON {
            Box::new(BoostQuery::new(query, *boost as Score))
        } else {
            query
        };
        should.push((Occur::Should, query));
    }
    if should.is_empty() {
        return Box::new(EmptyQuery);
    }

    let mut clauses: Vec<(Occur, Box<dyn Query>)> =
        vec![(Occur::Must, Box::new(BooleanQuery::new(should)))];
    if let Some(boosted) = title_prefix_boost(schema, q) {
        clauses.push((Occur::Should, Box::new(boosted)));
    }
    Box::new(BooleanQuery::new(clauses))
}

/// Boost `title_sort` exact/prefix matches for short queries.
fn title_prefix_boost(schema: &EntitySchema, q: &str) -> Option<BoostQuery> {
    let normalized = q.trim().to_lowercase();
    if normalized.is_empty() {
        return None;
    }
    let field = schema.title_sort_field();
    let exact = TermQuery::new(
        Term::from_field_text(field, &normalized),
        IndexRecordOption::Basic,
    );
    // Range [q, q\u{10FFFF}] expresses the prefix `q` on the raw fast field.
    let prefix = RangeQuery::new(
        Bound::Included(Term::from_field_text(field, &normalized)),
        Bound::Included(Term::from_field_text(
            field,
            &format!("{normalized}\u{10FFFF}"),
        )),
    );
    Some(BoostQuery::new(
        Box::new(BooleanQuery::new(vec![
            (
                Occur::Should,
                Box::new(BoostQuery::new(Box::new(exact), 5.0)) as Box<dyn Query>,
            ),
            (
                Occur::Should,
                Box::new(BoostQuery::new(Box::new(prefix), 2.0)) as Box<dyn Query>,
            ),
        ])),
        5.0,
    ))
}

/// `letter` filter: case-insensitive prefix on the display name.
///
/// PHP's Mongo `like` is case-sensitive; lowercasing both sides is a
/// deliberate, more useful approximation of "entries starting with the letter"
/// (the index stores the lowercase title in `title_sort` anyway).
fn letter_query(schema: &EntitySchema, letter: &str) -> Box<dyn Query> {
    let normalized = letter.trim().to_lowercase();
    if normalized.is_empty() {
        return Box::new(AllQuery);
    }
    let field = schema.title_sort_field();
    Box::new(RangeQuery::new(
        Bound::Included(Term::from_field_text(field, &normalized)),
        Bound::Included(Term::from_field_text(
            field,
            &format!("{normalized}\u{10FFFF}"),
        )),
    ))
}
// ---------------------------------------------------------------------------
// filters
// ---------------------------------------------------------------------------

fn add_filters(
    schema: &EntitySchema,
    params: &SearchParams,
    clauses: &mut Vec<(Occur, Box<dyn Query>)>,
) {
    // Club `type` is stored in the `access` field (`Club::filterByType`).
    let type_field = if schema.kind() == EntityKind::Club {
        "access"
    } else {
        "type"
    };
    add_eq_str(schema, clauses, type_field, params.media_type.as_deref());
    add_eq_str(schema, clauses, "status", params.status.as_deref());
    add_eq_str(schema, clauses, "rating", params.rating.as_deref());
    add_eq_str(schema, clauses, "category", params.category.as_deref());
    add_eq_str(schema, clauses, "gender", params.gender.as_deref());
    add_eq_str(schema, clauses, "location", params.location.as_deref());

    if let Some(score) = params.score {
        add_eq_f64(schema, clauses, "score", score);
    }
    // `filterByMinScore`/`filterByMaxScore`: ignore the "everything" bounds.
    if let Some(min) = params.min_score.filter(|value| *value != 0.0) {
        add_range_f64(schema, clauses, "score", Some(min), None);
    }
    if let Some(max) = params.max_score.filter(|value| *value != 10.0) {
        add_range_f64(schema, clauses, "score", None, Some(max));
    }

    if let Some(timestamp) = params.start_date.as_deref().and_then(parse_unix_day) {
        add_range_i64(schema, clauses, "start_date", Some(timestamp), None);
    }
    if let Some(timestamp) = params.end_date.as_deref().and_then(parse_unix_day) {
        add_range_i64(schema, clauses, "end_date", None, Some(timestamp));
    }

    // producer: (producers=X OR licensors=X OR studios=X) for any X.
    let producer_ids: Vec<u64> = params
        .producer
        .into_iter()
        .chain(parse_id_list_filtered_opt(params.producers.as_deref()).unwrap_or_default())
        .collect();
    if !producer_ids.is_empty() {
        let mut should: Vec<(Occur, Box<dyn Query>)> = Vec::new();
        for field in ["producers", "licensors", "studios"] {
            let Some(handle) = schema.field(field) else {
                continue;
            };
            for id in &producer_ids {
                should.push((
                    Occur::Should,
                    Box::new(TermQuery::new(
                        Term::from_field_u64(handle, *id),
                        IndexRecordOption::Basic,
                    )),
                ));
            }
        }
        if !should.is_empty() {
            clauses.push((Occur::Must, Box::new(BooleanQuery::new(should))));
        }
    }

    if let Some(id) = params.magazine {
        add_id_term(schema, clauses, "magazines", id);
    }
    if let Some(ids) = parse_id_list_filtered_opt(params.magazines.as_deref()) {
        add_id_any(schema, clauses, "magazines", &ids);
    }

    // genres: each id must appear in one of the genre-ish fields.
    if let Some(ids) = parse_id_list_opt(params.genres.as_deref()) {
        for id in ids {
            if let Some(query) = genre_any_query(schema, id) {
                clauses.push((Occur::Must, Box::new(query)));
            }
        }
    }
    // genres_exclude: none of the genre-ish fields may contain the id.
    if let Some(ids) = parse_id_list_opt(params.genres_exclude.as_deref()) {
        for id in ids {
            if let Some(query) = genre_any_query(schema, id) {
                clauses.push((Occur::MustNot, Box::new(query)));
            }
        }
    }

    if params.sfw == Some(true) {
        add_sfw(schema, clauses);
    }

    // `filterByUnapproved(true)` = include everything; otherwise approved only.
    if params.unapproved != Some(true) {
        add_eq_bool(schema, clauses, "approved", true);
    }

    match params.kids {
        Some(true) => add_id_term(schema, clauses, "demographics", KIDS_GENRE),
        Some(false) => add_id_term_not(schema, clauses, "demographics", KIDS_GENRE),
        None => {}
    }
}

/// `Constants::GENRE_ANIME/MANGA_HENTAI` and `..._EROTICA` (12/49) plus the
/// kids demographic (15). Same ids for anime and manga in jikan-php.
const HENTAI_GENRE: u64 = 12;
const KIDS_GENRE: u64 = 15;
const EROTICA_GENRE: u64 = 49;

/// `scopeExceptItemsWithAdultRating` for anime and manga.
fn add_sfw(schema: &EntitySchema, clauses: &mut Vec<(Occur, Box<dyn Query>)>) {
    match schema.kind() {
        EntityKind::Anime => {
            add_id_term_not(schema, clauses, "demographics", HENTAI_GENRE);
            add_id_term_not(schema, clauses, "demographics", EROTICA_GENRE);
            add_id_term_not(schema, clauses, "genres", HENTAI_GENRE);
            add_eq_str_not(schema, clauses, "rating", "Rx - Hentai");
        }
        EntityKind::Manga => {
            add_eq_str_not(schema, clauses, "type", "Doujinshi");
            add_id_term_not(schema, clauses, "demographics", HENTAI_GENRE);
            add_id_term_not(schema, clauses, "demographics", EROTICA_GENRE);
            add_id_term_not(schema, clauses, "genres", HENTAI_GENRE);
        }
        _ => {}
    }
}

/// `(genres = id OR themes = id OR demographics = id OR explicit_genres = id)`.
fn genre_any_query(schema: &EntitySchema, id: u64) -> Option<BooleanQuery> {
    let mut should: Vec<(Occur, Box<dyn Query>)> = Vec::new();
    for name in ["genres", "themes", "demographics", "explicit_genres"] {
        if let Some(field) = schema.field(name) {
            should.push((
                Occur::Should,
                Box::new(TermQuery::new(
                    Term::from_field_u64(field, id),
                    IndexRecordOption::Basic,
                )),
            ));
        }
    }
    if should.is_empty() {
        None
    } else {
        Some(BooleanQuery::new(should))
    }
}

fn add_eq_str(
    schema: &EntitySchema,
    clauses: &mut Vec<(Occur, Box<dyn Query>)>,
    field: &str,
    value: Option<&str>,
) {
    if let (Some(field), Some(value)) = (schema.field(field), value) {
        if !value.is_empty() {
            clauses.push((
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(field, value),
                    IndexRecordOption::Basic,
                )),
            ));
        }
    }
}

fn add_eq_str_not(
    schema: &EntitySchema,
    clauses: &mut Vec<(Occur, Box<dyn Query>)>,
    field: &str,
    value: &str,
) {
    if let Some(field) = schema.field(field) {
        clauses.push((
            Occur::MustNot,
            Box::new(TermQuery::new(
                Term::from_field_text(field, value),
                IndexRecordOption::Basic,
            )),
        ));
    }
}

fn add_eq_bool(
    schema: &EntitySchema,
    clauses: &mut Vec<(Occur, Box<dyn Query>)>,
    field: &str,
    value: bool,
) {
    if let Some(field) = schema.field(field) {
        clauses.push((
            Occur::Must,
            Box::new(TermQuery::new(
                Term::from_field_bool(field, value),
                IndexRecordOption::Basic,
            )),
        ));
    }
}

fn add_eq_f64(
    schema: &EntitySchema,
    clauses: &mut Vec<(Occur, Box<dyn Query>)>,
    field: &str,
    value: f64,
) {
    add_range_f64(schema, clauses, field, Some(value), Some(value));
}

fn add_range_f64(
    schema: &EntitySchema,
    clauses: &mut Vec<(Occur, Box<dyn Query>)>,
    field: &str,
    min: Option<f64>,
    max: Option<f64>,
) {
    if min.is_none() && max.is_none() {
        return;
    }
    let Some(field) = schema.field(field) else {
        return;
    };
    let lower = match min {
        Some(value) => Bound::Included(Term::from_field_f64(field, value)),
        None => Bound::Unbounded,
    };
    let upper = match max {
        Some(value) => Bound::Included(Term::from_field_f64(field, value)),
        None => Bound::Unbounded,
    };
    clauses.push((Occur::Must, Box::new(RangeQuery::new(lower, upper))));
}

fn add_range_i64(
    schema: &EntitySchema,
    clauses: &mut Vec<(Occur, Box<dyn Query>)>,
    field: &str,
    min: Option<i64>,
    max: Option<i64>,
) {
    if min.is_none() && max.is_none() {
        return;
    }
    let Some(field) = schema.field(field) else {
        return;
    };
    let lower = match min {
        Some(value) => Bound::Included(Term::from_field_i64(field, value)),
        None => Bound::Unbounded,
    };
    let upper = match max {
        Some(value) => Bound::Included(Term::from_field_i64(field, value)),
        None => Bound::Unbounded,
    };
    clauses.push((Occur::Must, Box::new(RangeQuery::new(lower, upper))));
}

fn add_id_term(
    schema: &EntitySchema,
    clauses: &mut Vec<(Occur, Box<dyn Query>)>,
    field: &str,
    id: u64,
) {
    if let Some(field) = schema.field(field) {
        clauses.push((
            Occur::Must,
            Box::new(TermQuery::new(
                Term::from_field_u64(field, id),
                IndexRecordOption::Basic,
            )),
        ));
    }
}

fn add_id_term_not(
    schema: &EntitySchema,
    clauses: &mut Vec<(Occur, Box<dyn Query>)>,
    field: &str,
    id: u64,
) {
    if let Some(field) = schema.field(field) {
        clauses.push((
            Occur::MustNot,
            Box::new(TermQuery::new(
                Term::from_field_u64(field, id),
                IndexRecordOption::Basic,
            )),
        ));
    }
}

/// Require at least one of `ids` in `field` (multi-valued fast field).
fn add_id_any(
    schema: &EntitySchema,
    clauses: &mut Vec<(Occur, Box<dyn Query>)>,
    field: &str,
    ids: &[u64],
) {
    let Some(field) = schema.field(field) else {
        return;
    };
    let should: Vec<(Occur, Box<dyn Query>)> = ids
        .iter()
        .map(|id| {
            (
                Occur::Should,
                Box::new(TermQuery::new(
                    Term::from_field_u64(field, *id),
                    IndexRecordOption::Basic,
                )) as Box<dyn Query>,
            )
        })
        .collect();
    if !should.is_empty() {
        clauses.push((Occur::Must, Box::new(BooleanQuery::new(should))));
    }
}

/// Parse `"1,2,3"` into ids, mirroring PHP's `(int)` cast for malformed parts
/// (which then filter against `mal_id 0`, i.e. match nothing).
fn parse_id_list(value: &str) -> Vec<u64> {
    if value.trim().is_empty() {
        return Vec::new();
    }
    value
        .split(',')
        .map(|part| part.trim().parse::<u64>().unwrap_or(0))
        .collect()
}

/// Like [`parse_id_list`], but for the filters where PHP calls `->filter()`
/// first (`producers`, `magazines`), dropping empty and `"0"` parts.
fn parse_id_list_filtered(value: &str) -> Vec<u64> {
    if value.trim().is_empty() {
        return Vec::new();
    }
    value
        .split(',')
        .filter(|part| {
            let part = part.trim();
            !part.is_empty() && part != "0"
        })
        .map(|part| part.trim().parse::<u64>().unwrap_or(0))
        .collect()
}

fn parse_id_list_opt(value: Option<&str>) -> Option<Vec<u64>> {
    let ids = parse_id_list(value?);
    (!ids.is_empty()).then_some(ids)
}

fn parse_id_list_filtered_opt(value: Option<&str>) -> Option<Vec<u64>> {
    let ids = parse_id_list_filtered(value?);
    (!ids.is_empty()).then_some(ids)
}

/// `YYYY-MM-DD` at 00:00 UTC, as `filterByStartDate`/`filterByEndDate` do.
fn parse_unix_day(value: &str) -> Option<i64> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
        .ok()?
        .and_hms_opt(0, 0, 0)
        .map(|datetime| datetime.and_utc().timestamp())
}

// ---------------------------------------------------------------------------
// sorting
// ---------------------------------------------------------------------------

/// Resolve the PHP `sort_by` construction into Tantivy sort keys.
fn sort_plan(schema: &EntitySchema, params: &SearchParams, has_text: bool) -> Vec<SortKeySpec> {
    let kind = schema.kind();
    let order_by = params
        .order_by
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let order_field = order_by.and_then(|name| schema.order_field(name));
    let direction = params.sort.unwrap_or(SortDirection::Asc);
    let short = has_text && is_short_query(params);

    let mut keys: Vec<SortKeySpec> = if has_text {
        match order_field {
            Some(field) => {
                if short {
                    // adaptToShortQueries moves _text_match to the front.
                    vec![SortKeySpec::Score, SortKeySpec::Field { field, direction }]
                } else {
                    // overrideSortingOrder: order_by first, then _text_match.
                    vec![SortKeySpec::Field { field, direction }, SortKeySpec::Score]
                }
            }
            None if short => vec![
                SortKeySpec::Score,
                SortKeySpec::Field {
                    field: schema.title_sort_field(),
                    direction: SortDirection::Asc,
                },
            ],
            None => {
                let mut keys = vec![SortKeySpec::Score];
                if matches!(kind, EntityKind::Anime | EntityKind::Manga) {
                    // getSearchIndexSortBy(): popularity asc, rank asc; sort=desc
                    // flips every `asc` in the pre-set sort_by.
                    keys.push(SortKeySpec::Field {
                        field: schema
                            .field("popularity")
                            .expect("media schema has popularity"),
                        direction,
                    });
                    keys.push(SortKeySpec::Field {
                        field: schema.field("rank").expect("media schema has rank"),
                        direction,
                    });
                }
                keys
            }
        }
    } else if let Some(field) = order_field {
        vec![SortKeySpec::Field { field, direction }]
    } else {
        // No q/order_by: `orderBy("mal_id")` asc; `sort` is ignored.
        vec![SortKeySpec::Field {
            field: schema.mal_id_field(),
            direction: SortDirection::Asc,
        }]
    };

    // Deterministic pagination on equal keys (Typesense ties are undefined).
    let mal_id = schema.mal_id_field();
    if !keys.iter().any(|key| match key {
        SortKeySpec::Field { field, .. } => *field == mal_id,
        SortKeySpec::Score => false,
    }) {
        keys.push(SortKeySpec::Field {
            field: mal_id,
            direction: SortDirection::Asc,
        });
    }

    keys
}

/// Run the collector for the resolved sort plan and return page addresses.
fn collect_top_docs(
    searcher: &Searcher,
    query: &dyn Query,
    keys: &[SortKeySpec],
    limit: usize,
    offset: usize,
) -> Result<Vec<DocAddress>> {
    let computers: Vec<(SortByErasedType, ComparatorEnum)> = keys
        .iter()
        .map(|key| match key {
            // Score is always descending (`_text_match:desc`).
            SortKeySpec::Score => (SortByErasedType::for_score(), ComparatorEnum::Natural),
            SortKeySpec::Field { field, direction } => (
                SortByErasedType::for_field(searcher.schema().get_field_name(*field)),
                if direction.is_desc() {
                    ComparatorEnum::Natural
                } else {
                    // Mongo puts null/missing first in ascending order.
                    ComparatorEnum::Reverse
                },
            ),
        })
        .collect();

    let hits: Vec<DocAddress> = match computers.len() {
        0 => {
            let top = TopDocs::with_limit(limit)
                .and_offset(offset)
                .order_by_score();
            searcher
                .search(query, &top)?
                .into_iter()
                .map(|(_score, address)| address)
                .collect()
        }
        1 => {
            let top = TopDocs::with_limit(limit)
                .and_offset(offset)
                .order_by(computers[0].clone());
            searcher
                .search(query, &top)?
                .into_iter()
                .map(|(_key, address)| address)
                .collect()
        }
        2 => {
            let top = TopDocs::with_limit(limit)
                .and_offset(offset)
                .order_by((computers[0].clone(), computers[1].clone()));
            searcher
                .search(query, &top)?
                .into_iter()
                .map(|(_key, address)| address)
                .collect()
        }
        3 => {
            let top = TopDocs::with_limit(limit).and_offset(offset).order_by((
                computers[0].clone(),
                (computers[1].clone(), computers[2].clone()),
            ));
            searcher
                .search(query, &top)?
                .into_iter()
                .map(|(_key, address)| address)
                .collect()
        }
        _ => {
            let top = TopDocs::with_limit(limit).and_offset(offset).order_by((
                computers[0].clone(),
                (
                    computers[1].clone(),
                    (computers[2].clone(), computers[3].clone()),
                ),
            ));
            searcher
                .search(query, &top)?
                .into_iter()
                .map(|(_key, address)| address)
                .collect()
        }
    };

    Ok(hits)
}

/// Load the stored payloads for the collected addresses.
fn fetch_payloads(
    searcher: &Searcher,
    schema: &EntitySchema,
    addresses: &[DocAddress],
) -> Result<Vec<Value>> {
    let mut items = Vec::with_capacity(addresses.len());
    for address in addresses {
        let document: TantivyDocument = searcher.doc(*address)?;
        let Some(raw) = document
            .get_first(schema.payload_field())
            .and_then(|value| value.as_str())
        else {
            continue;
        };
        let mut payload: Value = serde_json::from_str(raw)?;
        if let Some(object) = payload.as_object_mut() {
            // The paginator strips the Mongo `_id` from every item.
            object.remove("_id");
        }
        items.push(payload);
    }
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_id_lists() {
        assert_eq!(parse_id_list("1,2, 3"), vec![1, 2, 3]);
        assert_eq!(parse_id_list(""), Vec::<u64>::new());
        // PHP casts malformed parts with `(int)`, i.e. to 0.
        assert_eq!(parse_id_list("x,2"), vec![0, 2]);
        assert_eq!(parse_id_list("1,,2"), vec![1, 0, 2]);
        assert_eq!(parse_id_list_filtered("1,,2"), vec![1, 2]);
        assert_eq!(parse_id_list_filtered("1,0,2"), vec![1, 2]);
        assert_eq!(parse_id_list_filtered(""), Vec::<u64>::new());
    }

    #[test]
    fn parses_unix_days() {
        assert_eq!(parse_unix_day("2015-02-01"), Some(1422748800));
        assert_eq!(parse_unix_day("2012-05"), None);
        assert_eq!(parse_unix_day("garbage"), None);
    }

    #[test]
    fn text_search_detection_mirrors_query_builder() {
        let params = SearchParams::default();
        assert!(!ends_up_as_text_search(&params));

        let mut params = SearchParams {
            q: Some("cowboy".to_owned()),
            ..SearchParams::default()
        };
        assert!(ends_up_as_text_search(&params));

        params.letter = Some("c".to_owned());
        assert!(!ends_up_as_text_search(&params));

        params.q = Some("   ".to_owned());
        params.letter = None;
        assert!(!ends_up_as_text_search(&params));
    }

    #[test]
    fn default_limit_is_25_and_capped() {
        // Do not mutate the process environment from tests: assert the
        // constants and the clamping helper only.
        assert_eq!(DEFAULT_PER_PAGE, 25);
        assert_eq!(MAX_PER_PAGE_CEILING, 250);
    }
}
