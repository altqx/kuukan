//! Shared request-command bases and parse helpers.
//!
//! This module ports the abstract DTO classes of `app/Dto/*.php` together with
//! the shared concern traits of `app/Dto/Concerns/*` and
//! `App\Concerns\HasRequestFingerprint`. Concrete command structs live in the
//! family modules and embed these bases (Rust has no inheritance, so the PHP
//! `extends` relationship becomes a public field plus [`Deref`], which keeps
//! `cmd.limit` / `cmd.q` / `cmd.sfw` field access working).
//!
//! # Pipeline
//!
//! `spatie/laravel-data` 3.11 runs, in order:
//!
//! 1. `PreparesData::prepareForPipeline` — the trait used by
//!    [`SearchCommand`], [`MediaSearchCommand`], [`QueryAnimeSeasonCommand`],
//!    [`QueryTopItemsCommand`], [`QueryReviewsCommand`] and the seasonal/
//!    schedules/review-lookup commands. It pre-fills `limit` with
//!    `max_results_per_page()`, casts the strings `"true"`/`"false"` to booleans
//!    and drops empty strings for `Optional` non-bool properties.
//! 2. `MapPropertiesDataPipe` — copies mapped input names (`order_by`, `min_score`,
//!    `aired_from`, ...). Validation rules are keyed by the *input* name.
//! 3. `ValidatePropertiesDataPipe` — Laravel validation (400 on failure).
//! 4. `DefaultValuesDataPipe` — defaults for absent properties (`page = 1`,
//!    `sfw = false`, ...); `Optional` properties stay unset.
//! 5. `CastPropertiesDataPipe` — `EnumCast`, `ContextualBooleanCast`, dates.
//!
//! [`QueryParser`] reproduces that behavior: each `parse` helper records
//! validation messages in the order the Laravel rules run, returns a
//! best-effort value, and reports *cast* failures (which PHP surfaces as an
//! unhandled 500) as [`ApiError::Internal`]. Callers must check
//! [`QueryParser::finish`] **before** propagating the cast results so a
//! validation error always wins over a cast error, exactly like PHP.
//!
//! # Exact messages
//!
//! Jikan is a Lumen 9 app without a `resources/lang` directory, so Laravel
//! falls back to `laravel/lumen-framework`'s bundled `en/validation.php`
//! (v9.1.5). The wording differs from Laravel's default file, e.g.
//! `The limit must be a number.` instead of
//! `The limit field must be a number.`; these helpers reproduce the Lumen
//! strings.

use std::ops::Deref;

use chrono::Datelike;
use kuukan_core::enums::{
    EnumParseError, GenreFilter, MediaReviewsSort, SortDirection,
};
use kuukan_core::error::ApiError;
use kuukan_core::params::Query;
use kuukan_core::util::max_results_per_page;

use crate::dto::validation::Validator;

// ---------------------------------------------------------------------------
// Error helpers
// ---------------------------------------------------------------------------

/// PHP `str_replace('_', ' ', Str::snake($attribute))`: the displayable
/// attribute name used in validation messages (`minAge` → `min age`).
pub fn display_attribute(field: &str) -> String {
    let mut snake = String::with_capacity(field.len());
    let mut previous: Option<char> = None;
    for c in field.chars().filter(|c| !c.is_whitespace()) {
        if c.is_ascii_uppercase() && previous.is_some() {
            snake.push('_');
        }
        snake.push(c.to_ascii_lowercase());
        previous = Some(c);
    }
    snake.replace('_', " ")
}

/// PHP `empty()` for query strings: `""` and `"0"` are empty.
fn php_empty(raw: &str) -> bool {
    raw.is_empty() || raw == "0"
}

/// PHP `is_numeric()` (leading/trailing ASCII whitespace is ignored; the
/// accepted grammar is `[+-]?(\d+(\.\d*)?|\.\d+)([eE][+-]?\d+)?`).
pub fn php_is_numeric(raw: &str) -> bool {
    let bytes = raw.trim().as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let mut i = 0;
    if bytes[i] == b'+' || bytes[i] == b'-' {
        i += 1;
    }
    let mut digits = 0usize;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        digits += 1;
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            digits += 1;
            i += 1;
        }
    }
    if digits == 0 {
        return false;
    }
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        i += 1;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        let mut exponent = 0usize;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            exponent += 1;
            i += 1;
        }
        if exponent == 0 {
            return false;
        }
    }
    i == bytes.len()
}

/// PHP `filter_var($value, FILTER_VALIDATE_INT) !== false`.
pub fn php_is_integer(raw: &str) -> bool {
    let value = raw.trim();
    if value.is_empty() {
        return false;
    }
    let unsigned = value.strip_prefix(['+', '-']).unwrap_or(value);
    if unsigned.is_empty() || !unsigned.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    // FILTER_VALIDATE_INT rejects leading zeros such as "025" (octal lookalike).
    if unsigned.len() > 1 && unsigned.starts_with('0') {
        return false;
    }
    unsigned.parse::<i64>().is_ok()
}

/// PHP `(int) $value` for scalar strings.
pub fn php_intval(raw: &str) -> i64 {
    if php_is_numeric(raw) {
        raw.trim()
            .parse::<f64>()
            .map(|v| v as i64)
            .unwrap_or(0)
    } else {
        0
    }
}

/// Laravel `getSize()`: numeric values (when the attribute has a numeric rule)
/// return their numeric value, everything else its `mb_strlen`.
fn rule_size(raw: &str) -> f64 {
    if php_is_numeric(raw) {
        raw.trim().parse::<f64>().unwrap_or(0.0)
    } else {
        raw.chars().count() as f64
    }
}

/// Display value used by `lte`/`gte` `:value` replacements.
fn display_size(raw: &str) -> String {
    if php_is_numeric(raw) {
        raw.trim().to_string()
    } else {
        format!("{}", raw.chars().count())
    }
}

fn trim_number(value: f64) -> String {
    if value.fract().abs() < f64::EPSILON {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

/// A 500 raised while casting/constructing the DTO (PHP `TypeError` or
/// `CannotCastEnum` leaving the pipeline).
fn cast_error(error: impl Into<String>) -> ApiError {
    ApiError::Internal {
        message: "Unhandled Exception. Please follow report_url to generate an issue on GitHub"
            .to_string(),
        trace: None,
        error: Some(error.into()),
        report_url: None,
    }
}

fn enum_cast_error(php_class: &str) -> ApiError {
    cast_error(format!(
        "Could not cast enum: `` into a `{php_class}`"
    ))
}

/// The three enums with duplicate labels cannot resolve in PHP: `EnumRule`
/// fails, but building the validation message calls
/// `DuplicateLabelsException` which escapes as an unhandled 500 `Exception`
/// (not a ValidationException). Reproduce that status/type for parity.
fn is_broken_enum(php_class: &str) -> bool {
    matches!(
        php_class,
        "App\\Enums\\AnimeListAiringStatusFilterEnum"
            | "App\\Enums\\UserMangaListOrderByEnum"
            | "App\\Enums\\UserMangaListStatusFilterEnum"
    )
}

fn broken_enum_error(php_class: &str, value: &str) -> ApiError {
    cast_error(format!(
        "Spatie\\Enum\\Exceptions\\DuplicateLabelsException: Duplicate labels discovered for enum {php_class} while validating `{value}`"
    ))
}

fn int_cast_error(property: &str) -> ApiError {
    cast_error(format!(
        "Cannot assign string to property {property} of type Spatie\\LaravelData\\Optional|int"
    ))
}

// ---------------------------------------------------------------------------
// Date parsing (Carbon semantics for the cross-field rules)
// ---------------------------------------------------------------------------

/// A `Y-m-d` date. Stored as parts because the user-list MAL requests map
/// dates to `[year, month, day]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateOnly {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl DateOnly {
    pub fn new(year: i32, month: u32, day: u32) -> Option<Self> {
        chrono::NaiveDate::from_ymd_opt(year, month, day).map(|_| DateOnly { year, month, day })
    }

    /// Unix seconds at `00:00:00 UTC`.
    pub fn timestamp(&self) -> i64 {
        chrono::NaiveDate::from_ymd_opt(self.year, self.month, self.day)
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .map(|dt| dt.and_utc().timestamp())
            .unwrap_or(0)
    }

    /// `Y-m-d` rendering (PHP `Carbon::format('Y-m-d')`).
    pub fn to_ymd(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl std::fmt::Display for DateOnly {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_ymd())
    }
}

/// Strict PHP `date_format:Y-m-d` test: `createFromFormat('!Y-m-d', $value)`
/// plus `format('Y-m-d') == $value` (so `2020-1-2` is rejected).
pub fn php_date_format(raw: &str) -> bool {
    let b = raw.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    if !b
        .iter()
        .enumerate()
        .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
    {
        return false;
    }
    let year: i32 = raw[0..4].parse().unwrap_or(0);
    let month: u32 = raw[5..7].parse().unwrap_or(0);
    let day: u32 = raw[8..10].parse().unwrap_or(0);
    chrono::NaiveDate::from_ymd_opt(year, month, day).is_some()
}

/// Lenient date parsing used only by `before_or_equal` / `after_or_equal`
/// comparisons (`DateTime::createFromFormat('!Y-m-d', ...)` then
/// `Carbon::parse`, which is `strtotime`).
///
/// `""` parses as *now* (`Carbon::parse('')`), which matters when one side of a
/// user-list date range is an empty string.
pub fn php_lenient_date(raw: &str) -> Option<i64> {
    if raw.trim().is_empty() {
        // `Carbon::parse('')` and `Carbon::parse(' ')` both return *now*.
        return Some(chrono::Utc::now().timestamp());
    }
    if let Some(d) = parse_ymd_parts(raw) {
        return Some(d.timestamp());
    }
    parse_textual_date(raw).map(|d| d.timestamp())
}

/// `Y-m-d`, `Y/m/d`, `d-m-Y`, `m/d/Y` and unpadded variants.
fn parse_ymd_parts(raw: &str) -> Option<DateOnly> {
    let separators: &[char] = &['-', '/'];
    for sep in separators {
        let parts: Vec<&str> = raw.split(*sep).collect();
        if parts.len() != 3 {
            continue;
        }
        let nums: Option<Vec<i64>> = parts.iter().map(|p| p.parse::<i64>().ok()).collect();
        let Some(nums) = nums else { continue };
        let (a, b, c) = (nums[0], nums[1], nums[2]);
        if a > 31 {
            // Year first: Y-m-d / Y/m/d.
            if let Some(d) = DateOnly::new(a as i32, b as u32, c as u32) {
                return Some(d);
            }
            continue;
        }
        if *sep == '-' {
            // Dash separator: European d-m-Y.
            if let Some(d) = DateOnly::new(c as i32, b as u32, a as u32) {
                return Some(d);
            }
        } else {
            // Slash separator: American m/d/Y.
            if let Some(d) = DateOnly::new(c as i32, a as u32, b as u32) {
                return Some(d);
            }
        }
    }
    None
}

/// `"january 2 2020"`, `"2 january 2020"`, `"2 jan 2020"` style strings.
fn parse_textual_date(raw: &str) -> Option<DateOnly> {
    const MONTHS: [&str; 12] = [
        "january",
        "february",
        "march",
        "april",
        "may",
        "june",
        "july",
        "august",
        "september",
        "october",
        "november",
        "december",
    ];
    let tokens: Vec<String> = raw
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_ascii_lowercase())
        .collect();
    if tokens.len() != 3 {
        return None;
    }
    let month = tokens.iter().position(|t| {
        MONTHS
            .iter()
            .any(|m| m.starts_with(t.as_str()) && t.len() >= 3)
    })?;
    let numbers: Vec<i64> = tokens
        .iter()
        .filter_map(|t| t.parse::<i64>().ok())
        .collect();
    if numbers.len() != 2 {
        return None;
    }
    // `january 2 2020` and `2 january 2020` are both accepted: whatever the
    // token order, the two numbers are day then year.
    DateOnly::new(numbers[1] as i32, month as u32 + 1, numbers[0] as u32)
}

// ---------------------------------------------------------------------------
// Search commands
// ---------------------------------------------------------------------------

/// PHP `App\Dto\SearchCommand` (extended by every `/search` endpoint and the
/// user search). Carries `PreparesData`.
///
/// `letter` prohibits `q`, `q` is capped at 255 characters and `letter` must
/// be a single alphabetic character.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchCommand {
    pub q: Option<String>,
    pub sort: Option<SortDirection>,
    pub letter: Option<String>,
    pub page: u64,
    pub limit: u64,
}

impl SearchCommand {
    pub(crate) fn parse_with(parser: &mut QueryParser) -> Result<Self, ApiError> {
        let q = parser.string_max("q", 255);
        let sort = parser.enum_optional::<SortDirection>("sort", SortDirection::PHP_CLASS);
        let letter = parser.letter();
        let page = parser.page()?;
        let limit = parser.limit(None)?;
        // Field order in the PHP class is `q, sort, letter, page, limit`; the
        // prohibition is part of the `letter` rule list.
        parser.prohibits("letter", &["q"]);
        Ok(Self {
            q,
            sort: sort?,
            letter,
            page,
            limit,
        })
    }

    /// `parse` for a query-only command built on this base.
    pub fn parse(query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::clean(query);
        let command = Self::parse_with(&mut parser);
        parser.finish()?;
        command
    }
}

/// PHP `App\Dto\MediaSearchCommand` (base of the anime/manga searches).
#[derive(Debug, Clone, PartialEq)]
pub struct MediaSearchCommand {
    pub search: SearchCommand,
    pub sfw: bool,
    pub unapproved: bool,
    pub min_score: Option<f64>,
    pub max_score: Option<f64>,
    pub score: Option<f64>,
    pub genres: Option<String>,
    pub genres_exclude: Option<String>,
    pub start_date: Option<DateOnly>,
    pub end_date: Option<DateOnly>,
}

impl Deref for MediaSearchCommand {
    type Target = SearchCommand;

    fn deref(&self) -> &Self::Target {
        &self.search
    }
}

impl MediaSearchCommand {
    pub(crate) fn parse_with(parser: &mut QueryParser) -> Result<Self, ApiError> {
        let search = SearchCommand::parse_with(parser);
        let sfw = parser.bool_flag("sfw", false);
        let unapproved = parser.bool_flag("unapproved", false);
        // PHP class order: minScore, maxScore, score, genres, genresExclude,
        // start_date, end_date. `score` prohibits both score bounds.
        let min_score = parser.bounded_numeric("min_score", 0.0, 10.0)?;
        let max_score = parser.bounded_numeric("max_score", 1.0, 10.0)?;
        let score = parser.bounded_numeric("score", 1.0, 9.99)?;
        parser.prohibits("score", &["min_score", "max_score"]);
        let genres = parser.optional_string("genres");
        let genres_exclude = parser.optional_string("genres_exclude");
        let (start_date, end_date) = parser.date_range("start_date", "end_date");
        let min_raw = parser.get("min_score");
        let max_raw = parser.get("max_score");
        parser.score_cross_rules(min_raw, max_raw);
        Ok(Self {
            search: search?,
            sfw,
            unapproved,
            min_score,
            max_score,
            score,
            genres,
            genres_exclude,
            start_date,
            end_date,
        })
    }

    pub fn parse(query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::clean(query);
        let command = Self::parse_with(&mut parser);
        parser.finish()?;
        command
    }
}

// ---------------------------------------------------------------------------
// Season / top / reviews / user-list bases
// ---------------------------------------------------------------------------

/// PHP `App\Dto\QueryAnimeSeasonCommand` (base of `/seasons`, `/seasons/now`,
/// `/seasons/upcoming`). `kids` has no default and stays `Optional` when
/// absent.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryAnimeSeasonCommand {
    pub sfw: bool,
    pub kids: Option<bool>,
    pub unapproved: bool,
    pub limit: u64,
    pub page: u64,
    pub continuing: bool,
    pub filter: Option<kuukan_core::enums::AnimeType>,
}

impl HasRequestFingerprint for QueryAnimeSeasonCommand {}

impl QueryAnimeSeasonCommand {
    pub(crate) fn parse_with(parser: &mut QueryParser) -> Result<Self, ApiError> {
        let sfw = parser.bool_flag("sfw", false);
        let kids = parser.optional_bool_flag("kids");
        let unapproved = parser.bool_flag("unapproved", false);
        let limit = parser.limit(None)?;
        let page = parser.page()?;
        let continuing = parser.bool_flag("continuing", false);
        let filter = parser.enum_optional::<kuukan_core::enums::AnimeType>(
            "filter",
            kuukan_core::enums::AnimeType::PHP_CLASS,
        );
        Ok(Self {
            sfw,
            kids,
            unapproved,
            limit,
            page,
            continuing,
            filter: filter?,
        })
    }

    pub fn parse(query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::clean(query);
        let command = Self::parse_with(&mut parser);
        parser.finish()?;
        command
    }
}

/// PHP `App\Dto\QueryTopItemsCommand`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryTopItemsCommand {
    pub limit: u64,
    pub page: u64,
}

impl QueryTopItemsCommand {
    pub(crate) fn parse_with(parser: &mut QueryParser) -> Result<Self, ApiError> {
        let limit = parser.limit(None)?;
        let page = parser.page()?;
        Ok(Self { limit, page })
    }

    pub fn parse(query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::clean(query);
        let command = Self::parse_with(&mut parser);
        parser.finish()?;
        command
    }
}

/// PHP `App\Dto\QueryReviewsCommand`.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryReviewsCommand {
    pub page: u64,
    pub preliminary: Option<bool>,
    pub spoilers: Option<bool>,
    pub sort: Option<MediaReviewsSort>,
}

impl HasRequestFingerprint for QueryReviewsCommand {}

impl QueryReviewsCommand {
    pub(crate) fn parse_with(parser: &mut QueryParser) -> Result<Self, ApiError> {
        let page = parser.page()?;
        let preliminary = parser.optional_bool_flag("preliminary");
        let spoilers = parser.optional_bool_flag("spoilers");
        let sort = parser.enum_optional::<MediaReviewsSort>(
            "sort",
            MediaReviewsSort::PHP_CLASS,
        );
        Ok(Self {
            page,
            preliminary,
            spoilers,
            sort: sort?,
        })
    }

    pub fn parse(query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::clean(query);
        let command = Self::parse_with(&mut parser);
        parser.finish()?;
        command
    }

    /// `App\Features\Concerns\ResolvesMediaReviewParams::getReviewRequestParams()`:
    /// `sort` defaults to `mostVoted`, `spoilers`/`preliminary` to `false` and
    /// `page` to `1`.
    pub fn review_request_params(&self) -> ReviewRequestParams {
        ReviewRequestParams {
            sort: self.sort.unwrap_or(MediaReviewsSort::MostVoted),
            spoilers: self.spoilers.unwrap_or(false),
            preliminary: self.preliminary.unwrap_or(false),
            page: self.page,
        }
    }
}

/// Arguments resolved for the review repository/handler from a review command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReviewRequestParams {
    pub sort: MediaReviewsSort,
    pub spoilers: bool,
    pub preliminary: bool,
    pub page: u64,
}

/// PHP `App\Dto\QueryListOfUserCommand` (base of the anime/manga list
/// commands). It does **not** use `PreparesData`: empty strings stay in the
/// payload, which is why `?q=` keeps `Some("")` while `?aired_from=` trips the
/// explicit `Required` rule.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryListOfUserCommand {
    pub username: String,
    pub q: Option<String>,
    pub sort: Option<SortDirection>,
    pub page: u64,
}

impl HasRequestFingerprint for QueryListOfUserCommand {}

impl QueryListOfUserCommand {
    pub(crate) fn parse_with(parser: &mut QueryParser, username: &str) -> Result<Self, ApiError> {
        let username = parser.username_min3(username);
        let q = parser.string_max("q", 255);
        let sort = parser.enum_optional::<SortDirection>("sort", SortDirection::PHP_CLASS);
        let page = parser.page()?;
        Ok(Self {
            username,
            q,
            sort: sort?,
            page,
        })
    }
}

/// PHP `App\Dto\GenreListCommand`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenreListCommand {
    pub filter: Option<GenreFilter>,
}

impl GenreListCommand {
    pub(crate) fn parse_with(parser: &mut QueryParser) -> Result<Self, ApiError> {
        let filter =
            parser.enum_optional::<GenreFilter>("filter", GenreFilter::PHP_CLASS);
        Ok(Self { filter: filter? })
    }

    pub fn parse(query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::new(query);
        let command = Self::parse_with(&mut parser);
        parser.finish()?;
        command
    }
}

/// PHP `App\Dto\LookupDataCommand` (abstract base of the id lookups).
///
/// Concrete commands (`AnimeLookupCommand`, `ClubLookupCommand`, ...) are
/// generated by [`id_lookup_command`]; this type exists for callers that only
/// need the shared `id` behavior.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookupDataCommand {
    pub id: i64,
}

impl HasRequestFingerprint for LookupDataCommand {}

impl LookupDataCommand {
    pub fn parse(id: i64, query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::new(query);
        let id = parser.id(id);
        parser.finish()?;
        Ok(Self { id })
    }
}

/// PHP `App\Dto\LookupByUsernameCommand` (abstract base of the username
/// lookups).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookupByUsernameCommand {
    pub username: String,
}

impl HasRequestFingerprint for LookupByUsernameCommand {}

impl LookupByUsernameCommand {
    pub fn parse(username: &str, query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::new(query);
        let username = parser.username_lookup(username);
        parser.finish()?;
        Ok(Self { username })
    }
}

// ---------------------------------------------------------------------------
// Fingerprint
// ---------------------------------------------------------------------------

/// Port of `App\Concerns\HasRequestFingerprint`.
///
/// PHP stores the fingerprint computed while resolving the DTO from the
/// request (`HttpHelper::resolveRequestFingerprint($request)`); the HTTP layer
/// computes the same value here from the request URI.
pub trait HasRequestFingerprint {
    fn request_fingerprint(&self, request_uri: &str) -> String {
        request_fingerprint(request_uri)
    }
}

/// `request:<type>:<sha1(uri)>` for a `/v4/...` request URI
/// (`HttpHelper::resolveRequestFingerprint`).
pub fn request_fingerprint(request_uri: &str) -> String {
    kuukan_core::util::request_fingerprint(request_type(request_uri), request_uri)
}

/// `HttpHelper::requestType()`: second URI segment unless the first is a
/// version segment.
pub fn request_type(request_uri: &str) -> &str {
    let path = request_uri.split(['?', '#']).next().unwrap_or(request_uri);
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    match segments.as_slice() {
        [] => "",
        [first, rest @ ..] => {
            if matches!(*first, "v1" | "v2" | "v3" | "v4") {
                rest.first().copied().unwrap_or("")
            } else {
                first
            }
        }
    }
}

// ---------------------------------------------------------------------------
// QueryParser
// ---------------------------------------------------------------------------

/// Validation context for one command.
///
/// Validation messages are accumulated here (in the exact Laravel rule
/// order); cast errors are deferred so [`QueryParser::finish`] can report the
/// validation bag first, matching the PHP pipeline.
pub struct QueryParser<'a> {
    query: &'a Query,
    validator: Validator,
    clean: bool,
    max_limit: u64,
}

impl<'a> QueryParser<'a> {
    /// Context for classes that do **not** use `PreparesData`.
    pub fn new(query: &'a Query) -> Self {
        QueryParser {
            query,
            validator: Validator::new(),
            clean: false,
            max_limit: max_results_per_page(),
        }
    }

    /// Context for classes that use `PreparesData` (empty-string cleanup and
    /// `limit` pre-fill).
    pub fn clean(query: &'a Query) -> Self {
        QueryParser {
            clean: true,
            ..QueryParser::new(query)
        }
    }

    /// Finish validation: returns the 400 `ValidationException` body when any
    /// message was recorded.
    pub fn finish(self) -> Result<(), ApiError> {
        self.validator.finish()
    }

    /// Raw value exactly as received (last occurrence wins, PHP `parse_str`).
    pub fn get(&self, field: &str) -> Option<&'a str> {
        self.query.get(field)
    }

    pub fn has(&self, field: &str) -> bool {
        self.query.has(field)
    }

    /// The PHP maximum page size (`MAX_RESULTS_PER_PAGE`, default 25).
    pub fn max_limit(&self) -> u64 {
        self.max_limit
    }

    // -- message helpers ----------------------------------------------------

    fn add(&mut self, field: &str, message: String) {
        self.validator.add(field, message);
    }

    fn required(&mut self, field: &str) {
        self.add(
            field,
            format!("The {} field is required.", display_attribute(field)),
        );
    }

    fn numeric(&mut self, field: &str, raw: &str) {
        if !php_is_numeric(raw) {
            self.add(
                field,
                format!("The {} must be a number.", display_attribute(field)),
            );
        }
    }

    fn integer(&mut self, field: &str, raw: &str) {
        if !php_is_integer(raw) {
            self.add(
                field,
                format!("The {} must be an integer.", display_attribute(field)),
            );
        }
    }

    fn between_numeric(&mut self, field: &str, raw: &str, min: f64, max: f64) {
        let size = rule_size(raw);
        if size < min || size > max {
            self.add(
                field,
                format!(
                    "The {} must be between {} and {}.",
                    display_attribute(field),
                    trim_number(min),
                    trim_number(max)
                ),
            );
        }
    }

    fn min_numeric(&mut self, field: &str, raw: &str, min: f64) {
        if rule_size(raw) < min {
            self.add(
                field,
                format!(
                    "The {} must be at least {}.",
                    display_attribute(field),
                    trim_number(min)
                ),
            );
        }
    }

    fn max_numeric(&mut self, field: &str, raw: &str, max: f64) {
        if rule_size(raw) > max {
            self.add(
                field,
                format!(
                    "The {} must not be greater than {}.",
                    display_attribute(field),
                    trim_number(max)
                ),
            );
        }
    }

    fn max_string(&mut self, field: &str, raw: &str, max: usize) {
        if raw.chars().count() > max {
            self.add(
                field,
                format!(
                    "The {} must not be greater than {max} characters.",
                    display_attribute(field)
                ),
            );
        }
    }

    fn size_string(&mut self, field: &str, raw: &str, size: usize) {
        if raw.chars().count() != size {
            self.add(
                field,
                format!(
                    "The {} must be {size} characters.",
                    display_attribute(field)
                ),
            );
        }
    }

    fn alpha(&mut self, field: &str, raw: &str) {
        if !raw.chars().all(|c| c.is_alphabetic()) {
            self.add(
                field,
                format!(
                    "The {} must only contain letters.",
                    display_attribute(field)
                ),
            );
        }
    }

    fn date_format(&mut self, field: &str, raw: &str) {
        if !php_date_format(raw) {
            self.add(
                field,
                format!(
                    "The {} does not match the format Y-m-d.",
                    display_attribute(field)
                ),
            );
        }
    }

    fn before_or_equal(&mut self, field: &str, other_field: &str) {
        self.add(
            field,
            format!(
                "The {} must be a date before or equal to {}.",
                display_attribute(field),
                display_attribute(other_field)
            ),
        );
    }

    fn after_or_equal(&mut self, field: &str, other_field: &str) {
        self.add(
            field,
            format!(
                "The {} must be a date after or equal to {}.",
                display_attribute(field),
                display_attribute(other_field)
            ),
        );
    }

    fn lte(&mut self, field: &str, raw: &str, other: &str) {
        let ok = match (php_is_numeric(raw), php_is_numeric(other)) {
            (true, true) => {
                raw.trim().parse::<f64>().unwrap_or(0.0)
                    <= other.trim().parse::<f64>().unwrap_or(0.0)
            }
            _ => rule_size(raw) <= rule_size(other),
        };
        if !ok {
            self.add(
                field,
                format!(
                    "The {} must be less than or equal to {}.",
                    display_attribute(field),
                    display_size(other)
                ),
            );
        }
    }

    fn gte(&mut self, field: &str, raw: &str, other: &str) {
        let ok = match (php_is_numeric(raw), php_is_numeric(other)) {
            (true, true) => {
                raw.trim().parse::<f64>().unwrap_or(0.0)
                    >= other.trim().parse::<f64>().unwrap_or(0.0)
            }
            _ => rule_size(raw) >= rule_size(other),
        };
        if !ok {
            self.add(
                field,
                format!(
                    "The {} must be greater than or equal to {}.",
                    display_attribute(field),
                    display_size(other)
                ),
            );
        }
    }

    /// Laravel `prohibits`: this field fails when it is non-empty and any of
    /// `others` is non-empty. `others` are joined with `" / "`.
    pub fn prohibits(&mut self, field: &str, others: &[&str]) {
        let own_present = self
            .get(field)
            .map(|raw| !raw.trim().is_empty())
            .unwrap_or(false);
        if !own_present {
            return;
        }
        // Laravel replaces `:other` with *all* rule parameters, regardless of
        // which of them are actually present.
        let any_present = others.iter().any(|other| {
            self.get(other)
                .map(|raw| !raw.trim().is_empty())
                .unwrap_or(false)
        });
        if !any_present {
            return;
        }
        let names = others
            .iter()
            .map(|other| display_attribute(other))
            .collect::<Vec<_>>()
            .join(" / ");
        self.add(
            field,
            format!(
                "The {} field prohibits {names} from being present.",
                display_attribute(field)
            ),
        );
    }

    // -- scalar helpers -----------------------------------------------------

    /// `page`: `#[Numeric, Min(1)]` with default `1` and `PreparesData`
    /// removal of `?page=`.
    ///
    /// In classes without `PreparesData` an empty `page` skips validation and
    /// then fails the int property assignment, which PHP renders as a 500.
    pub fn page(&mut self) -> Result<u64, ApiError> {
        let Some(raw) = self.get("page") else {
            return Ok(1);
        };
        if raw.is_empty() && self.clean {
            // PreparesData drops the empty string, the default applies.
            return Ok(1);
        }
        if raw.trim().is_empty() {
            return Err(int_cast_error("page"));
        }
        self.numeric("page", raw);
        self.min_numeric("page", raw, 1.0);
        Ok(php_intval(raw).max(1) as u64)
    }

    /// `limit`: `#[IntegerType, Min(1), MaxLimitWithFallback]`, pre-filled by
    /// `PreparesData` with `max_results_per_page()`. `default` mirrors the
    /// (effectively unused) `$defaultLimit` static in PHP.
    pub fn limit(&mut self, default: Option<u64>) -> Result<u64, ApiError> {
        let fallback = default.unwrap_or(self.max_limit);
        let Some(raw) = self.get("limit") else {
            return Ok(fallback);
        };
        if raw.is_empty() {
            // PreparesData: `limit` was already pre-filled only when absent;
            // an empty string is dropped, which leaves the paginator default.
            return Ok(fallback);
        }
        if raw.trim().is_empty() {
            // Blank values skip the rules and then fail the int cast, which
            // PHP renders as a 500.
            return Err(int_cast_error("limit"));
        }
        self.numeric("limit", raw);
        self.integer("limit", raw);
        self.min_numeric("limit", raw, 1.0);
        let over = !php_is_numeric(raw) || php_intval(raw) > self.max_limit as i64;
        if over {
            self.add(
                "limit",
                format!(
                    "Value {raw} is higher than the configured '{}' max value.",
                    self.max_limit
                ),
            );
        }
        Ok(php_intval(raw).max(1) as u64)
    }

    /// Laravel `boolean` with `PreparesData` string conversion and the
    /// `ContextualBooleanCast` (an empty value becomes `true`).
    pub fn bool_flag(&mut self, field: &str, default: bool) -> bool {
        match self.get(field) {
            None => default,
            Some(raw) => self.cast_bool(field, raw),
        }
    }

    /// Nullable boolean (`kids`, `preliminary`, `spoilers`): absent stays
    /// `None`, empty becomes `true`.
    pub fn optional_bool_flag(&mut self, field: &str) -> Option<bool> {
        self.get(field).map(|raw| self.cast_bool(field, raw))
    }

    fn cast_bool(&mut self, field: &str, raw: &str) -> bool {
        if raw.trim().is_empty() {
            // All non-implicit rules are skipped for empty values; the
            // contextual cast turns `""` into `true`.
            return true;
        }
        match raw {
            "true" | "1" => true,
            "false" | "0" => false,
            _ => {
                self.add(
                    field,
                    format!(
                        "The {} field must be true or false.",
                        display_attribute(field)
                    ),
                );
                false
            }
        }
    }

    /// `type|Optional $enum` with `EnumCast`: absent (or an empty value in a
    /// `PreparesData` class) is `None`; anything else must resolve or the PHP
    /// cast fails with a 500.
    pub fn enum_optional<T: std::str::FromStr<Err = EnumParseError>>(
        &mut self,
        field: &str,
        php_class: &'static str,
    ) -> Result<Option<T>, ApiError> {
        let raw = self.get(field);
        self.enum_optional_raw(field, raw, php_class)
    }

    /// `enum_optional` with an explicit raw value (route parameter).
    pub fn enum_optional_raw<T: std::str::FromStr<Err = EnumParseError>>(
        &mut self,
        field: &str,
        raw: Option<&str>,
        php_class: &'static str,
    ) -> Result<Option<T>, ApiError> {
        let Some(raw) = raw else {
            return Ok(None);
        };
        if raw.is_empty() && self.clean {
            return Ok(None);
        }
        if raw.trim().is_empty() {
            // Empty values skip the EnumRule, then `EnumCast::cast()` throws.
            return Err(enum_cast_error(php_class));
        }
        match raw.parse::<T>() {
            Ok(value) => Ok(Some(value)),
            Err(_) => {
                if is_broken_enum(php_class) {
                    return Err(broken_enum_error(php_class, raw));
                }
                self.validator
                    .enum_rule(field, raw, &[], php_class);
                Ok(None)
            }
        }
    }

    /// `?Enum $enum` (nullable, not optional): absent is `null`; an empty
    /// value skips the rule and then fails the cast.
    pub fn enum_nullable<T: std::str::FromStr<Err = EnumParseError>>(
        &mut self,
        field: &str,
        php_class: &'static str,
    ) -> Result<Option<T>, ApiError> {
        let raw = self.get(field);
        self.enum_nullable_raw(field, raw, php_class)
    }

    /// `enum_nullable` with an explicit raw value (route parameter).
    pub fn enum_nullable_raw<T: std::str::FromStr<Err = EnumParseError>>(
        &mut self,
        field: &str,
        raw: Option<&str>,
        php_class: &'static str,
    ) -> Result<Option<T>, ApiError> {
        let Some(raw) = raw else {
            return Ok(None);
        };
        if raw.trim().is_empty() {
            // Empty values skip the `EnumRule`; `EnumCast` then throws because
            // nullable properties have no `Optional` guard.
            return Err(enum_cast_error(php_class));
        }
        match raw.parse::<T>() {
            Ok(value) => Ok(Some(value)),
            Err(_) => {
                if is_broken_enum(php_class) {
                    return Err(broken_enum_error(php_class, raw));
                }
                self.validator.enum_rule(field, raw, &[], php_class);
                Ok(None)
            }
        }
    }

    /// A non-nullable enum path parameter (`season`): `#[Required, EnumRule]`.
    pub fn enum_required<T: std::str::FromStr<Err = EnumParseError>>(
        &mut self,
        field: &str,
        raw: Option<&str>,
        php_class: &'static str,
    ) -> Option<T> {
        let Some(raw) = raw else {
            self.required(field);
            return None;
        };
        if raw.trim().is_empty() {
            self.required(field);
            return None;
        }
        match raw.parse::<T>() {
            Ok(value) => Some(value),
            Err(_) => {
                self.validator.enum_rule(field, raw, &[], php_class);
                None
            }
        }
    }

    /// `string|Optional` with `#[Max(255), StringType]` (`q`) — the value is
    /// only checked for length; empty is removed by `PreparesData`.
    pub fn string_max(&mut self, field: &str, max: usize) -> Option<String> {
        let raw = self.optional_string(field)?;
        if !raw.trim().is_empty() {
            self.max_string(field, &raw, max);
        }
        Some(raw)
    }

    /// `letter`: `#[Size(1), StringType, Alpha, Prohibits("q")]`.
    fn letter(&mut self) -> Option<String> {
        let raw = self.optional_string("letter")?;
        if raw.trim().is_empty() {
            return Some(raw);
        }
        self.size_string("letter", &raw, 1);
        self.alpha("letter", &raw);
        Some(raw)
    }

    /// `string|Optional` without validation attributes: empty is removed in
    /// `PreparesData` classes, kept otherwise.
    pub fn optional_string(&mut self, field: &str) -> Option<String> {
        let raw = self.get(field)?;
        if raw.is_empty() && self.clean {
            return None;
        }
        Some(raw.to_string())
    }

    /// `int|Optional` with `#[Min(1)]` (`producer`, `magazine`).
    pub fn int_min(&mut self, field: &str, min: f64) -> Result<Option<i64>, ApiError> {
        let Some(raw) = self.int_raw(field, field)? else {
            return Ok(None);
        };
        if !raw.trim().is_empty() {
            self.numeric(field, &raw);
            self.min_numeric(field, &raw, min);
        }
        Ok(Some(php_intval(&raw)))
    }

    /// `int|Optional` with `#[IntegerType, Min(1)]` (`producer` on anime).
    pub fn int_min_integer(
        &mut self,
        field: &str,
        min: f64,
    ) -> Result<Option<i64>, ApiError> {
        let Some(raw) = self.int_raw(field, field)? else {
            return Ok(None);
        };
        if !raw.trim().is_empty() {
            self.numeric(field, &raw);
            self.integer(field, &raw);
            self.min_numeric(field, &raw, min);
        }
        Ok(Some(php_intval(&raw)))
    }

    /// `int|Optional` with `#[Min, Max]` (`year` on user lists).
    pub fn int_min_max(
        &mut self,
        field: &str,
        min: f64,
        max: f64,
    ) -> Result<Option<i64>, ApiError> {
        let Some(raw) = self.int_raw(field, field)? else {
            return Ok(None);
        };
        if !raw.trim().is_empty() {
            self.numeric(field, &raw);
            self.min_numeric(field, &raw, min);
            self.max_numeric(field, &raw, max);
        }
        Ok(Some(php_intval(&raw)))
    }

    /// Numeric-only `int|Optional` (`minAge`, `maxAge`).
    pub fn numeric_int(&mut self, field: &str) -> Result<Option<i64>, ApiError> {
        let Some(raw) = self.int_raw(field, field)? else {
            return Ok(None);
        };
        if !raw.trim().is_empty() {
            self.numeric(field, &raw);
        }
        Ok(Some(php_intval(&raw)))
    }

    /// Shared `Optional int` handling: returns `None` for absent values,
    /// errors when PHP would fail the int cast, and otherwise hands back the
    /// raw string for rule checks.
    fn int_raw(
        &mut self,
        field: &str,
        property: &str,
    ) -> Result<Option<String>, ApiError> {
        let Some(raw) = self.get(field) else {
            return Ok(None);
        };
        if raw.is_empty() && self.clean {
            return Ok(None);
        }
        if raw.trim().is_empty() || (raw.is_empty() && !self.clean) {
            // Empty values skip the rules, then the int property cast fails.
            return Err(int_cast_error(property));
        }
        Ok(Some(raw.to_string()))
    }

    /// `float|Optional` with `#[Between, Numeric]` (score filters).
    pub fn bounded_numeric(
        &mut self,
        field: &str,
        min: f64,
        max: f64,
    ) -> Result<Option<f64>, ApiError> {
        let Some(raw) = self.get(field) else {
            return Ok(None);
        };
        if raw.is_empty() && self.clean {
            return Ok(None);
        }
        if raw.trim().is_empty() {
            // Blank values skip the rules and then fail the float cast, which
            // PHP renders as a 500.
            return Err(int_cast_error(field));
        }
        self.between_numeric(field, raw, min, max);
        self.numeric(field, raw);
        Ok(raw.trim().parse::<f64>().ok())
    }

    /// `withValidator()` cross rules for `min_score`/`max_score`: added only
    /// when the opposite bound is truthy (`empty()` excludes `""` and `"0"`).
    pub fn score_cross_rules(&mut self, min_raw: Option<&str>, max_raw: Option<&str>) {
        let min_present = min_raw.map(|v| !php_empty(v)).unwrap_or(false);
        let max_present = max_raw.map(|v| !php_empty(v)).unwrap_or(false);
        if let Some(min) = min_raw {
            if !min.trim().is_empty() && max_present {
                let other = max_raw.unwrap_or_default();
                self.lte("min_score", min, other);
            }
        }
        if let Some(max) = max_raw {
            if !max.trim().is_empty() && min_present {
                let other = min_raw.unwrap_or_default();
                self.gte("max_score", max, other);
            }
        }
    }

    /// `start_date`/`end_date` (or `aired_from`/`aired_to`,
    /// `published_from`/`published_to`).
    ///
    /// Every date field carries an explicit `#[Required]` attribute, so a
    /// blank (whitespace-only) value fails `required`; `PreparesData` classes
    /// drop an exact `""` before validation instead.
    pub fn date_range(
        &mut self,
        from_field: &str,
        to_field: &str,
    ) -> (Option<DateOnly>, Option<DateOnly>) {
        let from_raw = self.date_value(from_field);
        let to_raw = self.date_value(to_field);

        if let Some(raw) = from_raw.as_deref() {
            if raw.trim().is_empty() {
                self.required(from_field);
            } else {
                if let Some(other) = to_raw.as_deref() {
                    let ok = match (php_lenient_date(raw), php_lenient_date(other)) {
                        (Some(a), Some(b)) => a <= b,
                        _ => false,
                    };
                    if !ok {
                        self.before_or_equal(from_field, to_field);
                    }
                }
                self.date_format(from_field, raw);
            }
        }

        if let Some(raw) = to_raw.as_deref() {
            if raw.trim().is_empty() {
                self.required(to_field);
            } else {
                if let Some(other) = from_raw.as_deref() {
                    let ok = match (php_lenient_date(raw), php_lenient_date(other)) {
                        (Some(a), Some(b)) => a >= b,
                        _ => false,
                    };
                    if !ok {
                        self.after_or_equal(to_field, from_field);
                    }
                }
                self.date_format(to_field, raw);
            }
        }

        (
            from_raw.as_deref().and_then(php_lenient_date).map(to_date_only),
            to_raw.as_deref().and_then(php_lenient_date).map(to_date_only),
        )
    }

    /// Raw presence for a date field: `None` when absent (or dropped as an
    /// empty string by `PreparesData`).
    fn date_value(&self, field: &str) -> Option<String> {
        let raw = self.get(field)?;
        if raw.is_empty() && self.clean {
            return None;
        }
        Some(raw.to_string())
    }

    /// `username` from `QueryListOfUserCommand` (`#[Min(3)]`, string rules).
    pub fn username_min3(&mut self, username: &str) -> String {
        if username.is_empty() {
            self.required("username");
        } else if username.chars().count() < 3 {
            self.add(
                "username",
                "The username must be at least 3 characters.".to_string(),
            );
        }
        username.to_string()
    }

    /// `username` from `LookupByUsernameCommand`
    /// (`#[StringType, Max(255), Min(3)]`).
    pub fn username_lookup(&mut self, username: &str) -> String {
        if username.is_empty() {
            self.required("username");
        } else {
            if username.chars().count() > 255 {
                self.add(
                    "username",
                    "The username must not be greater than 255 characters.".to_string(),
                );
            }
            if username.chars().count() < 3 {
                self.add(
                    "username",
                    "The username must be at least 3 characters.".to_string(),
                );
            }
        }
        username.to_string()
    }

    /// `id` from `LookupDataCommand` (`#[Numeric, Required, Min(1)]`). The
    /// route patterns only match digits, so only `Min(1)` can fail.
    pub fn id(&mut self, id: i64) -> i64 {
        if id < 1 {
            self.add("id", "The id must be at least 1.".to_string());
        }
        id
    }

    /// `episodeId` (`#[Numeric, Required, Min(1)]`).
    pub fn episode_id(&mut self, episode_id: i64) -> i64 {
        if episode_id < 1 {
            self.add(
                "episodeId",
                "The episode id must be at least 1.".to_string(),
            );
        }
        episode_id
    }

    /// `year` on `QuerySpecificAnimeSeasonCommand`:
    /// `#[Required, Between(1000, 2999)]`.
    pub fn year(&mut self, year: i64) -> i64 {
        if !(1000..=2999).contains(&year) {
            self.add(
                "year",
                "The year must be between 1000 and 2999.".to_string(),
            );
        }
        year
    }

}

/// Records the search-handler `q` control-character rejection
/// (`SearchRequestHandler::handle`). Runs after DTO resolution, so validation
/// must already have succeeded.
pub fn check_search_q(q: Option<&str>) -> Result<(), ApiError> {
    const MESSAGE: &str =
        "The q parameter cannot contain any of the following characters: \\n, \\r, \\t, \\0, %0A";
    let Some(q) = q else {
        return Ok(());
    };
    let forbidden = ["\n", "\\n", "\r", "\t", "\0", "%0A"];
    if forbidden.iter().any(|needle| q.contains(needle)) {
        let mut validator = Validator::new();
        validator.add("q", MESSAGE.to_string());
        return Err(validator.finish().unwrap_err());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Lookup command macros
// ---------------------------------------------------------------------------

/// Defines a `LookupDataCommand` subclass: a single route `id`.
macro_rules! id_lookup_command {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name {
            pub id: i64,
        }

        impl $name {
            /// Parse the route id (`#[Numeric, Required, Min(1)]`).
            pub fn parse(
                id: i64,
                query: &kuukan_core::params::Query,
            ) -> Result<Self, kuukan_core::error::ApiError> {
                let mut parser = $crate::dto::base::QueryParser::new(query);
                let id = parser.id(id);
                parser.finish()?;
                Ok(Self { id })
            }
        }

        impl $crate::dto::base::HasRequestFingerprint for $name {}
    };
}
pub(crate) use id_lookup_command;

/// Defines a `LookupDataCommand` subclass with a paginated response.
macro_rules! id_page_lookup_command {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name {
            pub id: i64,
            pub page: u64,
        }

        impl $name {
            /// Parse the route id and the `page` query parameter.
            pub fn parse(
                id: i64,
                query: &kuukan_core::params::Query,
            ) -> Result<Self, kuukan_core::error::ApiError> {
                let mut parser = $crate::dto::base::QueryParser::new(query);
                let id = parser.id(id);
                let page = parser.page()?;
                parser.finish()?;
                Ok(Self { id, page })
            }
        }

        impl $crate::dto::base::HasRequestFingerprint for $name {}
    };
}
pub(crate) use id_page_lookup_command;

/// Defines a `LookupByUsernameCommand` subclass.
macro_rules! username_lookup_command {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name {
            pub username: String,
        }

        impl $name {
            /// Parse the route username (`#[StringType, Max(255), Min(3)]`).
            pub fn parse(
                username: &str,
                query: &kuukan_core::params::Query,
            ) -> Result<Self, kuukan_core::error::ApiError> {
                let mut parser = $crate::dto::base::QueryParser::new(query);
                let username = parser.username_lookup(username);
                parser.finish()?;
                Ok(Self { username })
            }
        }

        impl $crate::dto::base::HasRequestFingerprint for $name {}
    };
}
pub(crate) use username_lookup_command;

/// Defines a paginated `LookupByUsernameCommand` subclass.
macro_rules! username_page_lookup_command {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name {
            pub username: String,
            pub page: u64,
        }

        impl $name {
            /// Parse the route username and the `page` query parameter.
            pub fn parse(
                username: &str,
                query: &kuukan_core::params::Query,
            ) -> Result<Self, kuukan_core::error::ApiError> {
                let mut parser = $crate::dto::base::QueryParser::new(query);
                let username = parser.username_lookup(username);
                let page = parser.page()?;
                parser.finish()?;
                Ok(Self { username, page })
            }
        }

        impl $crate::dto::base::HasRequestFingerprint for $name {}
    };
}
pub(crate) use username_page_lookup_command;

fn to_date_only(timestamp: i64) -> DateOnly {
    match chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp, 0) {
        Some(datetime) => {
            let date = datetime.date_naive();
            DateOnly {
                year: date.year(),
                month: date.month(),
                day: date.day(),
            }
        }
        None => DateOnly {
            year: 1970,
            month: 1,
            day: 1,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn messages(err: ApiError) -> serde_json::Value {
        err.body(false)["messages"].clone()
    }

    #[test]
    fn display_attribute_matches_laravel_snake() {
        assert_eq!(display_attribute("start_date"), "start date");
        assert_eq!(display_attribute("minAge"), "min age");
        assert_eq!(display_attribute("episodeId"), "episode id");
        assert_eq!(display_attribute("order_by2"), "order by2");
        assert_eq!(display_attribute("q"), "q");
    }

    #[test]
    fn php_number_rules_match_filters() {
        assert!(php_is_numeric("25"));
        assert!(php_is_numeric(" 25 "));
        assert!(php_is_numeric("+25"));
        assert!(php_is_numeric(".5"));
        assert!(php_is_numeric("5."));
        assert!(!php_is_numeric("abc"));
        assert!(!php_is_numeric(""));
        assert!(!php_is_numeric("INF"));
        assert!(!php_is_numeric("0x1A"));

        assert!(php_is_integer("25"));
        assert!(php_is_integer(" 25 "));
        assert!(php_is_integer("-25"));
        assert!(!php_is_integer("025"));
        assert!(!php_is_integer("25.0"));
        assert!(!php_is_integer("abc"));
        assert!(!php_is_integer(""));
    }

    #[test]
    fn limit_messages_follow_rule_order() {
        let query = Query::from_pairs([("limit", "abc")]);
        let mut parser = QueryParser::clean(&query);
        let _ = parser.limit(None);
        let bag = messages(parser.finish().unwrap_err());
        assert_eq!(
            bag["limit"],
            serde_json::json!([
                "The limit must be a number.",
                "The limit must be an integer.",
                "Value abc is higher than the configured '25' max value."
            ])
        );

        let query = Query::from_pairs([("limit", "999")]);
        let mut parser = QueryParser::clean(&query);
        let _ = parser.limit(None);
        let bag = messages(parser.finish().unwrap_err());
        assert_eq!(
            bag["limit"],
            serde_json::json!(["Value 999 is higher than the configured '25' max value."])
        );
    }

    #[test]
    fn page_messages_match_php() {
        let query = Query::from_pairs([("page", "abc")]);
        let mut parser = QueryParser::clean(&query);
        parser.page().unwrap();
        let bag = messages(parser.finish().unwrap_err());
        assert_eq!(bag["page"], serde_json::json!(["The page must be a number."]));

        let query = Query::from_pairs([("page", "0")]);
        assert_eq!(QueryParser::clean(&query).page().unwrap(), 1);
        let mut parser = QueryParser::clean(&query);
        parser.page().unwrap();
        let bag = messages(parser.finish().unwrap_err());
        assert_eq!(
            bag["page"],
            serde_json::json!(["The page must be at least 1."])
        );
    }

    #[test]
    fn bool_empty_is_true_like_contextual_cast() {
        let query = Query::from_pairs([("sfw", "")]);
        let mut parser = QueryParser::clean(&query);
        assert!(parser.bool_flag("sfw", false));
        parser.finish().unwrap();

        let query = Query::from_pairs([("sfw", "maybe")]);
        let mut parser = QueryParser::clean(&query);
        assert!(!parser.bool_flag("sfw", false));
        let bag = messages(parser.finish().unwrap_err());
        assert_eq!(
            bag["sfw"],
            serde_json::json!(["The sfw field must be true or false."])
        );
    }

    #[test]
    fn prohibits_joins_field_names() {
        let query = Query::from_pairs([("score", "5"), ("min_score", "1"), ("max_score", "9")]);
        let mut parser = QueryParser::clean(&query);
        parser.prohibits("score", &["min_score", "max_score"]);
        let bag = messages(parser.finish().unwrap_err());
        assert_eq!(
            bag["score"],
            serde_json::json!(["The score field prohibits min score / max score from being present."])
        );
    }

    #[test]
    fn date_format_is_strict() {
        assert!(php_date_format("2020-01-02"));
        assert!(!php_date_format("2020-1-2"));
        assert!(!php_date_format("2020-13-01"));
        assert!(!php_date_format("2020-02-30"));
        assert!(!php_date_format(""));
    }

    #[test]
    fn lookup_base_commands_validate_route_params() {
        assert_eq!(LookupDataCommand::parse(1, &Query::new()).unwrap().id, 1);
        let err = LookupDataCommand::parse(0, &Query::new()).unwrap_err();
        assert_eq!(err.body(false)["messages"]["id"][0], "The id must be at least 1.");

        assert_eq!(
            LookupByUsernameCommand::parse("nekomata", &Query::new())
                .unwrap()
                .username,
            "nekomata"
        );
        let err = LookupByUsernameCommand::parse("ab", &Query::new()).unwrap_err();
        assert_eq!(
            err.body(false)["messages"]["username"][0],
            "The username must be at least 3 characters."
        );
    }

    #[test]
    fn request_type_skips_version_segment() {
        assert_eq!(request_type("/v4/anime/1"), "anime");
        assert_eq!(request_type("/anime/1"), "anime");
        assert_eq!(request_type("/v4/anime?q=x"), "anime");
    }
}
