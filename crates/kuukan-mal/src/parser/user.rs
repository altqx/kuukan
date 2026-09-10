//! Port of `Jikan\Parser\User\**` (jikan-php v4.0.12).
//!
//! Contains the parsers for the user profile, friends, history, clubs,
//! username-by-id, the `load.json` anime/manga list item factories and the
//! user-scoped review/recommendation/recently-online pages that `MalClient`
//! delegates to non-user parser classes in PHP.
//!
//! Review items are parsed by `parser::reviews`, recommendation items by
//! `parser::recommendations` and the recently-online list by `parser::search`;
//! this module only owns the user-specific wrappers and pagination.

use chrono::{DateTime, FixedOffset};
use regex::Regex;
use serde_json::{json, Value};
use std::sync::OnceLock;

use crate::error::ParseError;
use crate::parser::date::{
    format_atom, parse_date, parse_date_dmy, parse_date_mdy, parse_date_mdy_readable,
    parse_date_time_pst,
};
use crate::parser::helper::{parse_image_quality, parse_image_thumb_to_hq, HtmlDoc, HtmlNode};
use crate::parser::jstring::{cleanse, str_to_canonical, utf8_nbsp_trim};
use crate::parser::mal_url::{club_id_from_url, id_from_url, MalUrl, BASE_URL};
use crate::parser::recommendations::RecommendationListItemParser;
use crate::parser::reviews::{AnimeReviewParser, MangaReviewParser};

// ---------------------------------------------------------------------------
// shared value helpers
// ---------------------------------------------------------------------------

/// Returned when a node/text the PHP parser reads unconditionally is missing
/// (PHP throws `InvalidArgumentException` there, which `MalClient` wraps in a
/// `ParserException`).
fn missing(xpath: &str) -> ParseError {
    ParseError::InvalidXPath(format!("no node matched {xpath}"))
}

fn required_first(node: &HtmlNode, xpath: &str) -> Result<HtmlNode, ParseError> {
    node.first(xpath)?.ok_or_else(|| missing(xpath))
}

fn required_text(node: &HtmlNode, xpath: &str) -> Result<String, ParseError> {
    Ok(required_first(node, xpath)?.node_text())
}

fn optional_text(node: &HtmlNode, xpath: &str) -> Result<Option<String>, ParseError> {
    Ok(node.first(xpath)?.map(|n| n.node_text()))
}

fn required_attr(node: &HtmlNode, xpath: &str, name: &str) -> Result<String, ParseError> {
    required_first(node, xpath)?
        .node_attr(name)
        .ok_or_else(|| missing(&format!("{xpath}/@{name}")))
}

fn optional_attr(node: &HtmlNode, xpath: &str, name: &str) -> Result<Option<String>, ParseError> {
    Ok(node.first(xpath)?.and_then(|n| n.node_attr(name)))
}

/// PHP `trim()` default char list.
fn php_trim(value: &str) -> String {
    value
        .trim_matches(|c| matches!(c, ' ' | '\t' | '\n' | '\r' | '\0' | '\u{0B}'))
        .to_string()
}

/// PHP `(int) $string`: optional sign then leading digits.
fn php_int(value: &str) -> i64 {
    let bytes = value.as_bytes();
    let mut i = 0;
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C) {
        i += 1;
    }
    let negative = if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        let neg = bytes[i] == b'-';
        i += 1;
        neg
    } else {
        false
    };
    let start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if start == i {
        return 0;
    }
    let value = value[start..i].parse::<i64>().unwrap_or(i64::MAX);
    if negative {
        -value
    } else {
        value
    }
}

/// PHP `(int)` cast of a JSON value.
fn php_int_value(value: &Value) -> i64 {
    match value {
        Value::Number(number) => number
            .as_i64()
            .or_else(|| number.as_f64().map(|f| f as i64))
            .unwrap_or(0),
        Value::Bool(true) => 1,
        Value::Bool(false) | Value::Null => 0,
        Value::String(string) => php_int(string),
        _ => 0,
    }
}

/// PHP `(bool)` cast of a JSON value.
fn php_bool(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(number) => number.as_f64().map(|f| f != 0.0).unwrap_or(false),
        Value::String(string) => !string.is_empty() && string != "0",
        Value::Array(array) => !array.is_empty(),
        Value::Object(_) => true,
    }
}

/// PHP `empty($value)`: `!$value` for every JSON-representable value.
fn php_empty(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => true,
        Some(value) => !php_bool(value),
    }
}

/// Raw JSON field (missing/`null` -> JSON `null`), mirroring PHP's
/// untyped-property assignment of an absent `$item->field`.
fn raw_field(item: &Value, name: &str) -> Value {
    item.get(name).cloned().unwrap_or(Value::Null)
}

/// `(string) $item->field`, treating absent/null as PHP's `""` (concatenation
/// context) but preserving non-string scalars.
fn string_field(item: &Value, name: &str) -> Option<String> {
    match item.get(name) {
        None | Some(Value::Null) => None,
        Some(Value::String(value)) => Some(value.clone()),
        Some(other) => Some(other.to_string()),
    }
}

/// `(bool) $item->field` where a missing property becomes `null` (untyped
/// property), i.e. keep the raw JSON value.
fn optional_bool_field(item: &Value, name: &str) -> Value {
    item.get(name).cloned().unwrap_or(Value::Null)
}

fn date_value(value: Option<DateTime<FixedOffset>>) -> Value {
    match value {
        Some(value) => Value::String(format_atom(&value)),
        None => Value::Null,
    }
}

// ---------------------------------------------------------------------------
// image resources
// ---------------------------------------------------------------------------

/// `Jikan\Model\Resource\UserImageResource\UserImageResource`.
///
/// PHP quirk: a `null` URL still runs `str_replace` for the webp URL, which
/// coerces `null` to `""`.
fn user_image_resource(image_url: Option<&str>) -> Value {
    let jpg = json!({ "image_url": image_url });
    let webp_url = match image_url {
        Some(url) => Value::String(url.replace(".jpg", ".webp")),
        None => Value::String(String::new()),
    };
    json!({
        "jpg": jpg,
        "webp": { "image_url": webp_url },
    })
}

/// `Jikan\Model\Resource\CommonImageResource\CommonImageResource`.
///
/// PHP quirk: the jpg factory returns early on `null` (small/large stay null)
/// while the webp factory only treats `image_url === null` as null, so the
/// `str_replace` on `null` yields `""` for all three fields.
fn common_image_resource(image_url: Option<&str>) -> Value {
    let jpg = match image_url {
        Some(url) => json!({
            "image_url": url,
            "small_image_url": url.replace(".jpg", "t.jpg"),
            "large_image_url": url.replace(".jpg", "l.jpg"),
        }),
        None => json!({
            "image_url": null,
            "small_image_url": null,
            "large_image_url": null,
        }),
    };
    let webp = match image_url {
        Some(url) => json!({
            "image_url": url.replace(".jpg", ".webp"),
            "small_image_url": url.replace(".jpg", "t.webp"),
            "large_image_url": url.replace(".jpg", "l.webp"),
        }),
        None => json!({
            "image_url": "",
            "small_image_url": "",
            "large_image_url": "",
        }),
    };
    json!({ "jpg": jpg, "webp": webp })
}

/// `Jikan\Model\Resource\CharacterImageResource\CharacterImageResource`.
fn character_image_resource(image_url: Option<&str>) -> Value {
    let jpg = json!({ "image_url": image_url });
    let webp = match image_url {
        Some(url) => json!({
            "image_url": url.replace(".jpg", ".webp"),
            "small_image_url": url.replace(".jpg", "t.webp"),
        }),
        None => json!({
            "image_url": "",
            "small_image_url": "",
        }),
    };
    json!({ "jpg": jpg, "webp": webp })
}

/// `Jikan\Model\Resource\PersonImageResource\PersonImageResource`.
fn person_image_resource(image_url: Option<&str>) -> Value {
    json!({ "jpg": { "image_url": image_url } })
}

/// `Jikan\Model\Common\CommonMeta` / `AnimeMeta` / `MangaMeta` JMS shape.
///
/// The PHP meta constructors run `Parser::parseImageQuality()` themselves, so
/// callers passing an already-processed URL are unaffected (the operation is
/// idempotent).
fn common_meta(title: &str, url: &str, image_url: &str) -> Value {
    let image_url = parse_image_quality(image_url);
    json!({
        "mal_id": id_from_url(url),
        "url": url,
        "images": common_image_resource(Some(&image_url)),
        "title": title,
    })
}

// ---------------------------------------------------------------------------
// Jikan\Parser\User\Profile\UserProfileParser
// ---------------------------------------------------------------------------

/// `Jikan\Parser\User\Profile\UserProfileParser`.
pub struct UserProfileParser {
    node: HtmlNode,
}

impl UserProfileParser {
    pub fn new(doc: &HtmlDoc) -> Self {
        UserProfileParser { node: doc.root() }
    }

    /// `getUserId()`: first `id=` match in the report link.
    pub fn get_user_id(&self) -> Result<Option<i64>, ParseError> {
        let Some(node) = self.node.first("//a[contains(@class, 'header-right')]")? else {
            return Ok(None);
        };
        let href = node.node_attr("href").unwrap_or_default();
        Ok(user_id_re().captures(&href).map(|caps| php_int(&caps[1])))
    }

    /// `getProfileUrl()`.
    pub fn get_profile_url(&self) -> Result<String, ParseError> {
        self.node
            .attr("//meta[@property=\"og:url\"]", "content")?
            .ok_or_else(|| missing("//meta[@property=\"og:url\"]"))
    }

    /// `getUsername()`: `preg_replace('#.*/(.*)$#', '$1', $url)`.
    pub fn get_username(&self) -> Result<String, ParseError> {
        let url = self.get_profile_url()?;
        Ok(url.rsplit('/').next().unwrap_or("").to_string())
    }

    /// `getImageUrl()`: null when the image node is absent.
    pub fn get_image_url(&self) -> Result<Option<String>, ParseError> {
        optional_attr(
            &self.node,
            "//div[contains(@class, \"user-image\")]/img",
            "data-src",
        )
    }

    /// `getJoinDate()`.
    pub fn get_join_date(&self) -> Result<Option<DateTime<FixedOffset>>, ParseError> {
        let text = required_text(
            &self.node,
            "//span[contains(text(), 'Joined')]/following-sibling::span",
        )?;
        Ok(parse_date_mdy_readable(&text))
    }

    /// `getLastOnline()` (MAL time is `America/Los_Angeles`).
    pub fn get_last_online(&self) -> Result<Option<DateTime<FixedOffset>>, ParseError> {
        let text = required_text(
            &self.node,
            "//span[contains(text(), 'Last Online')]/following-sibling::span",
        )?;
        Ok(parse_date_time_pst(&text))
    }

    /// `getGender()`.
    pub fn get_gender(&self) -> Result<Option<String>, ParseError> {
        optional_text(
            &self.node,
            "//ul[contains(@class, \"user-status\")]/li/span[contains(text(), \"Gender\")]/following-sibling::span",
        )
    }

    /// `getBirthday()`.
    pub fn get_birthday(&self) -> Result<Option<DateTime<FixedOffset>>, ParseError> {
        let Some(node) = self
            .node
            .first("//span[contains(text(), 'Birthday')]/following-sibling::span")?
        else {
            return Ok(None);
        };
        Ok(parse_date_mdy_readable(&node.node_text()))
    }

    /// `getLocation()`.
    pub fn get_location(&self) -> Result<Option<String>, ParseError> {
        optional_text(
            &self.node,
            "//ul[contains(@class, \"user-status\")]/li/span[contains(text(), \"Location\")]/following-sibling::span",
        )
    }

    /// `getAbout()`: `trim(html())` of the about block, `null` when absent.
    pub fn get_about(&self) -> Result<Option<String>, ParseError> {
        match self
            .node
            .first("//div[@class='profile-about-user js-truncate-inner']/table/tr/td/div")?
        {
            Some(node) => Ok(Some(php_trim(&node.node_html()))),
            None => Ok(None),
        }
    }

    /// `getAnimeStats()`.
    pub fn get_anime_stats(&self) -> Result<Value, ParseError> {
        anime_stats(&self.node)
    }

    /// `getMangaStats()`.
    pub fn get_manga_stats(&self) -> Result<Value, ParseError> {
        manga_stats(&self.node)
    }

    /// `getFavorites()`.
    pub fn get_favorites(&self) -> Result<Value, ParseError> {
        let nodes = self
            .node
            .nodes("//div[contains(@class, 'container-right')]")?;
        favorites(&nodes)
    }

    /// `getUserLastUpdates()`.
    pub fn get_user_last_updates(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "anime": last_updates(&self.node, "anime")?,
            "manga": last_updates(&self.node, "manga")?,
        }))
    }

    /// `getUserExternalLinks()`.
    pub fn get_user_external_links(&self) -> Result<Vec<Value>, ParseError> {
        let nodes = self.node.nodes(
            "//*[@id=\"content\"]/div/div[1]/div/div[contains(@class, \"user-profile-sns\")][1]/a",
        )?;
        let mut links = Vec::with_capacity(nodes.len());
        for node in &nodes {
            links.push(crate::parser::common::url_parser(node)?);
        }
        Ok(links)
    }

    /// `Profile::fromParser()` — the whole JMS payload.
    pub fn get_model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "mal_id": self.get_user_id()?,
            "username": self.get_username()?,
            "url": self.get_profile_url()?,
            "images": user_image_resource(self.get_image_url()?.as_deref()),
            "last_online": date_value(self.get_last_online()?),
            "gender": self.get_gender()?,
            "birthday": date_value(self.get_birthday()?),
            "location": self.get_location()?,
            "joined": date_value(self.get_join_date()?),
            "anime_stats": self.get_anime_stats()?,
            "manga_stats": self.get_manga_stats()?,
            "favorites": self.get_favorites()?,
            "last_updates": self.get_user_last_updates()?,
            "external_links": self.get_user_external_links()?,
            "about": self.get_about()?,
        }))
    }
}

fn user_id_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"id=(.*)").expect("valid regex"))
}

// --- AnimeStatsParser / MangaStatsParser -----------------------------------

fn comma_int(node: &HtmlNode, xpath: &str) -> Result<Option<i64>, ParseError> {
    Ok(node
        .first(xpath)?
        .map(|n| php_int(&n.node_text().replace(',', ""))))
}

fn comma_float(node: &HtmlNode, xpath: &str) -> Result<Option<f64>, ParseError> {
    let Some(node) = node.first(xpath)? else {
        return Ok(None);
    };
    // PHP `Parser::removeChildNodes($node)->text()`: the "Days: " label span is
    // removed before the number is read.
    node.remove_child_nodes()?;
    Ok(Some(
        node.node_text()
            .replace(',', "")
            .trim()
            .parse::<f64>()
            .unwrap_or(0.0),
    ))
}

/// `Jikan\Parser\User\Profile\AnimeStatsParser::getModel()`.
fn anime_stats(node: &HtmlNode) -> Result<Value, ParseError> {
    let mean_score = match node.first(
        "//*[@id=\"statistics\"]/div[contains(@class, \"user-statistics-stats\")][1]/div[contains(@class, \"stats anime\")]/div[1]/div[2]/span[contains(@class, \"score-label\")]",
    )? {
        Some(score) => score
            .node_text()
            .trim()
            .parse::<f64>()
            .unwrap_or(0.0),
        None => comma_float(node, "//div[@class='di-tc ar pr8 fs12 fw-b'][1]")?.unwrap_or(0.0),
    };

    Ok(json!({
        "days_watched": comma_float(node, "//div[@class='di-tc al pl8 fs12 fw-b'][1]")?.unwrap_or(0.0),
        "mean_score": mean_score,
        "watching": comma_int(node, "//a[contains(@class, 'watching')]/following-sibling::span")?,
        "completed": comma_int(node, "//a[contains(@class, 'completed')]/following-sibling::span")?,
        "on_hold": comma_int(node, "//a[contains(@class, 'on_hold')]/following-sibling::span")?,
        "dropped": comma_int(node, "//a[contains(@class, 'dropped')]/following-sibling::span")?,
        "plan_to_watch": comma_int(node, "//a[contains(@class, 'plan_to_watch')]/following-sibling::span")?,
        "total_entries": comma_int(node, "//ul[@class='stats-data fl-r'][1]/li[1]/span[2]")?,
        "rewatched": comma_int(node, "//ul[@class='stats-data fl-r'][1]/li[2]/span[2]")?,
        "episodes_watched": comma_int(node, "//ul[@class='stats-data fl-r'][1]/li[3]/span[2]")?,
    }))
}

/// `Jikan\Parser\User\Profile\MangaStatsParser::getModel()`.
fn manga_stats(node: &HtmlNode) -> Result<Value, ParseError> {
    let mean_score = match node.first(
        "//*[@id=\"statistics\"]/div[contains(@class, \"user-statistics-stats\")][2]/div[contains(@class, \"stats manga\")]/div[1]/div[2]/span[contains(@class, \"score-label\")]",
    )? {
        Some(score) => score
            .node_text()
            .trim()
            .parse::<f64>()
            .unwrap_or(0.0),
        None => comma_float(node, "//div[@class='di-tc ar pr8 fs12 fw-b'][2]")?.unwrap_or(0.0),
    };

    Ok(json!({
        "days_read": comma_float(node, "//div[@class='di-tc al pl8 fs12 fw-b'][2]")?.unwrap_or(0.0),
        "mean_score": mean_score,
        "reading": comma_int(node, "//a[contains(@class, 'reading')]/following-sibling::span")?,
        "completed": comma_int(node, "//a[contains(@class, 'completed')][2]/following-sibling::span")?,
        "on_hold": comma_int(node, "//a[contains(@class, 'on_hold')][2]/following-sibling::span")?,
        "dropped": comma_int(node, "//a[contains(@class, 'dropped')][2]/following-sibling::span")?,
        "plan_to_read": comma_int(node, "//a[contains(@class, 'plan_to_read')]/following-sibling::span")?,
        "total_entries": comma_int(node, "//ul[@class='stats-data fl-r'][2]/li[1]/span[2]")?,
        "reread": comma_int(node, "//ul[@class='stats-data fl-r'][2]/li[2]/span[2]")?,
        "chapters_read": comma_int(node, "//ul[@class='stats-data fl-r'][2]/li[3]/span[2]")?,
        "volumes_read": comma_int(node, "//ul[@class='stats-data fl-r'][2]/li[4]/span[2]")?,
    }))
}

// --- FavoritesParser --------------------------------------------------------

/// Collect the matches of `xpath` from every context node (DomCrawler
/// evaluates `filterXPath()` per node of the selection and concatenates).
fn collect_nodes(nodes: &[HtmlNode], xpath: &str) -> Result<Vec<HtmlNode>, ParseError> {
    let mut out = Vec::new();
    for node in nodes {
        out.extend(node.nodes(xpath)?);
    }
    Ok(out)
}

/// `Jikan\Parser\User\Profile\FavoritesParser`.
fn favorites(nodes: &[HtmlNode]) -> Result<Value, ParseError> {
    Ok(json!({
        "anime": favorite_entries(
            nodes,
            "//div[@id='anime_favorites']/div[@class='fav-slide-outer']/ul/li"
        )?,
        "manga": favorite_entries(
            nodes,
            "//div[@id='manga_favorites']/div[@class='fav-slide-outer']/ul/li"
        )?,
        "characters": character_favorites(
            nodes,
            "//div[@id='character_favorites']/div[@class='fav-slide-outer']/ul/li"
        )?,
        "people": person_favorites(
            nodes,
            "//div[@id='person_favorites']/div[@class='fav-slide-outer']/ul/li"
        )?,
    }))
}

/// `FavoriteAnime`/`FavoriteManga` (`FavoriteListEntry`).
fn favorite_entries(nodes: &[HtmlNode], xpath: &str) -> Result<Vec<Value>, ParseError> {
    let mut entries = Vec::new();
    for item in collect_nodes(nodes, xpath)? {
        let title = required_text(&item, "//a/span[contains(@class, 'title')]")?;
        let url = required_attr(&item, "//a", "href")?;
        let image = parse_image_quality(&required_attr(&item, "//a/img", "data-src")?);
        let type_and_year = required_text(&item, "//a/span[contains(@class, 'users')]")?;
        let (entity_type, year) = split_type_and_year(&type_and_year);
        entries.push(json!({
            "mal_id": id_from_url(&url),
            "url": url,
            "images": common_image_resource(Some(&image)),
            "title": title,
            "type": entity_type,
            "start_year": year,
        }));
    }
    Ok(entries)
}

fn split_type_and_year(type_and_year: &str) -> (String, i64) {
    let mut parts = type_and_year.split('\u{B7}');
    let entity_type = parts.next().unwrap_or("").trim().to_string();
    let year = parts.next().map(|part| php_int(part.trim())).unwrap_or(0);
    (entity_type, year)
}

/// `CharacterMeta` favourites.
fn character_favorites(nodes: &[HtmlNode], xpath: &str) -> Result<Vec<Value>, ParseError> {
    let mut entries = Vec::new();
    for item in collect_nodes(nodes, xpath)? {
        let name = required_text(&item, "//a/span[contains(@class, 'title')]")?;
        let url = format!("{BASE_URL}{}", required_attr(&item, "//a", "href")?);
        let image = parse_image_quality(&required_attr(&item, "//a/img", "data-src")?);
        entries.push(json!({
            "mal_id": id_from_url(&url),
            "url": url,
            "images": character_image_resource(Some(&image)),
            "name": name,
        }));
    }
    Ok(entries)
}

/// `PersonMeta` favourites.
fn person_favorites(nodes: &[HtmlNode], xpath: &str) -> Result<Vec<Value>, ParseError> {
    let mut entries = Vec::new();
    for item in collect_nodes(nodes, xpath)? {
        let name = required_text(&item, "//a/span[contains(@class, 'title')]")?;
        let url = format!("{BASE_URL}{}", required_attr(&item, "//a", "href")?);
        let image = parse_image_quality(&required_attr(&item, "//a/img", "data-src")?);
        entries.push(json!({
            "mal_id": id_from_url(&url),
            "url": url,
            "images": person_image_resource(Some(&image)),
            "name": name,
        }));
    }
    Ok(entries)
}

// --- LastUpdatesParser ------------------------------------------------------

struct BaseLastUpdate {
    url: String,
    title: String,
    image_url: String,
    progressed: Option<i64>,
    total: Option<i64>,
    status: String,
    score: i64,
    date: Option<DateTime<FixedOffset>>,
}

/// `Jikan\Parser\User\Profile\LastUpdatesParser::parseBaseListUpdates()`.
fn base_last_updates(
    node: &HtmlNode,
    update_type: &str,
) -> Result<Vec<BaseLastUpdate>, ParseError> {
    let xpath = format!("//div[contains(@class, 'updates {update_type}')]/div");
    let mut updates = Vec::new();
    for item in node.nodes(&xpath)? {
        let anchor = required_first(&item, "//a")?;
        let image = required_first(&anchor, "//img")?;
        let title = image.node_attr("alt").unwrap_or_default();
        let url = anchor.node_attr("href").unwrap_or_default();
        let image_url = image.node_attr("data-src").unwrap_or_default();
        let date = parse_date(&required_text(&item, "//div/div[1]/span")?);

        let text = required_text(&item, "//div/div[2]")?;
        let scored = match text.find("Scored") {
            Some(index) => text[index + "Scored".len()..].trim().to_string(),
            None => {
                // PHP: strpos() === false + strlen('Scored') = 6.
                text.chars().skip(6).collect::<String>().trim().to_string()
            }
        };
        let score = if !scored.is_empty() && scored.bytes().all(|b| b.is_ascii_digit()) {
            php_int(&scored)
        } else {
            0
        };

        let progress_type_value = text.split('\u{B7}').next().unwrap_or("").to_string();
        // PHP `strpos(...) != false` is a loose comparison: a `/` at position 0
        // counts as "not found".
        let separator = progress_type_value.find('/').filter(|index| *index != 0);
        updates.push(if separator.is_some() {
            let progressed = progress_re()
                .captures(&progress_type_value)
                .and_then(|caps| caps.get(1))
                .map(|m| php_int(m.as_str()));
            let total = progress_re()
                .captures(&progress_type_value)
                .and_then(|caps| caps.get(2))
                .map(|m| php_int(m.as_str()));
            let status = status_re()
                .captures(&progress_type_value)
                .and_then(|caps| caps.get(1))
                .map(|m| cleanse(m.as_str()))
                .unwrap_or_default();
            BaseLastUpdate {
                url,
                title,
                image_url,
                progressed,
                total,
                status,
                score,
                date,
            }
        } else {
            let stripped = progress_type_value.replace(" -", "");
            BaseLastUpdate {
                url,
                title,
                image_url,
                progressed: None,
                total: None,
                status: php_trim(&stripped),
                score,
                date,
            }
        });
    }
    Ok(updates)
}

fn progress_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(\d+)/(\d+)").expect("valid regex"))
}

fn status_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"([a-zA-Z\s\-]+)").expect("valid regex"))
}

/// `LastUpdates::fromParser()` (`anime` = `LastAnimeUpdate`, `manga` =
/// `LastMangaUpdate`).
fn last_updates(node: &HtmlNode, update_type: &str) -> Result<Vec<Value>, ParseError> {
    let mut updates = Vec::new();
    for update in base_last_updates(node, update_type)? {
        let entry = common_meta(&update.title, &update.url, &update.image_url);
        let value = if update_type == "anime" {
            json!({
                "entry": entry,
                "score": update.score,
                "status": update.status,
                "episodes_seen": update.progressed,
                "episodes_total": update.total,
                "date": date_value(update.date),
            })
        } else {
            json!({
                "entry": entry,
                "score": update.score,
                "status": update.status,
                "chapters_read": update.progressed,
                "chapters_total": update.total,
                "date": date_value(update.date),
            })
        };
        updates.push(value);
    }
    Ok(updates)
}

// ---------------------------------------------------------------------------
// Jikan\Parser\User\Friends\**
// ---------------------------------------------------------------------------

/// `Jikan\Parser\User\Friends\FriendParser`.
pub struct FriendParser {
    node: HtmlNode,
}

impl FriendParser {
    pub fn new(node: HtmlNode) -> Self {
        FriendParser { node }
    }

    /// `getAvatar()` (`str_replace(['thumbs/', '_thumb'], '', ...)`).
    pub fn get_avatar(&self) -> Result<String, ParseError> {
        Ok(parse_image_thumb_to_hq(&required_attr(
            &self.node,
            "//div/a/img",
            "data-src",
        )?))
    }

    /// `getName()`.
    pub fn get_name(&self) -> Result<String, ParseError> {
        required_text(&self.node, "//div[3]/div/a")
    }

    /// `getUrl()`.
    pub fn get_url(&self) -> Result<String, ParseError> {
        required_attr(&self.node, "//div[3]/div/a", "href")
    }

    /// `getFriendsSince()`.
    pub fn get_friends_since(&self) -> Result<Option<DateTime<FixedOffset>>, ParseError> {
        let Some(node) = self
            .node
            .first("//div[contains(@class, \"data\")]/div[3]")?
        else {
            return Ok(None);
        };
        let text = cleanse(&node.node_text());
        let Some(caps) = friends_since_re().captures(&text) else {
            return Ok(None);
        };
        Ok(parse_date(&caps[1]))
    }

    /// `getLastOnline()` (`new \DateTimeImmutable($text, UTC)`).
    pub fn get_last_online(&self) -> Result<DateTime<FixedOffset>, ParseError> {
        let text = cleanse(&required_text(
            &self.node,
            "//div[contains(@class, \"data\")]/div[2]",
        )?);
        parse_date(&text).ok_or_else(|| missing("//div[contains(@class, \"data\")]/div[2]"))
    }

    /// `getUserMeta()`: `{username, url, images}`.
    pub fn get_user_meta(&self) -> Result<Value, ParseError> {
        let avatar = parse_image_quality(&self.get_avatar()?);
        Ok(json!({
            "username": self.get_name()?,
            "url": self.get_url()?,
            "images": user_image_resource(Some(&avatar)),
        }))
    }

    /// `Friend::fromParser()`.
    pub fn get_model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "user": self.get_user_meta()?,
            "last_online": format_atom(&self.get_last_online()?),
            "friends_since": date_value(self.get_friends_since()?),
        }))
    }
}

fn friends_since_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^Friends since (.*)$").expect("valid regex"))
}

/// `Jikan\Parser\User\Friends\FriendsParser`.
pub struct FriendsParser {
    node: HtmlNode,
}

impl FriendsParser {
    pub fn new(doc: &HtmlDoc) -> Self {
        FriendsParser { node: doc.root() }
    }

    /// `getResults()`.
    pub fn get_results(&self) -> Result<Vec<Value>, ParseError> {
        let nodes = self.node.nodes(
            "//div[contains(@class, \"boxlist-container\")]/div[contains(@class, \"boxlist\")]",
        )?;
        let mut results = Vec::with_capacity(nodes.len());
        for node in nodes {
            results.push(FriendParser::new(node).get_model()?);
        }
        Ok(results)
    }

    /// `getLastPage()`.
    pub fn get_last_page(&self) -> Result<i64, ParseError> {
        let Some(page) = self.node.first(
            "//*[@id=\"content\"]/table/tr/td[2]/div[2]/div[contains(@class, \"mt12 mb12\")]/div[contains(@class, \"pagination\")]",
        )? else {
            return Ok(1);
        };
        let links = page.nodes("//a[contains(@class, \"link\")]")?;
        let Some(last) = links.last() else {
            return Err(missing("//a[contains(@class, \"link\")]"));
        };
        let href = last.node_attr("href").unwrap_or_default();
        match offset_re().captures(&href) {
            Some(caps) => Ok(php_int(&caps[1]) / 100 + 1),
            None => Ok(1),
        }
    }

    /// `getHasNextPage()`.
    pub fn get_has_next_page(&self) -> Result<bool, ParseError> {
        Ok(!self
            .node
            .nodes("//*[@id=\"content\"]/div/div[2]/div/div[2]//a[text()=\"Next\"]")?
            .is_empty())
    }

    /// `Friends::fromParser()`.
    pub fn get_model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "results": self.get_results()?,
            "has_next_page": self.get_has_next_page()?,
            "last_visible_page": self.get_last_page()?,
        }))
    }
}

fn offset_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\?offset=(\d+)$").expect("valid regex"))
}

// ---------------------------------------------------------------------------
// Jikan\Parser\User\History\**
// ---------------------------------------------------------------------------

/// `Jikan\Parser\User\History\HistoryParser`.
pub struct HistoryParser {
    node: HtmlNode,
}

impl HistoryParser {
    pub fn new(doc: &HtmlDoc) -> Self {
        HistoryParser { node: doc.root() }
    }

    /// `HistoryParser::getModel()`.
    pub fn get_model(&self) -> Result<Vec<Value>, ParseError> {
        let rows = self.node.nodes("//div[@id=\"content\"]/div/table/tr")?;
        let mut history = Vec::new();
        for row in rows {
            if row.count("//td[contains(@class, \"borderClass\")]")? == 0 {
                continue;
            }
            history.push(history_item(&row)?);
        }
        Ok(history)
    }
}

fn history_item(row: &HtmlNode) -> Result<Value, ParseError> {
    let anchor = required_first(row, "//td[1]/a")?;
    let name = cleanse(&anchor.node_text());
    let href = anchor.node_attr("href").unwrap_or_default();
    let caps = history_url_re()
        .captures(&href)
        .ok_or_else(|| missing("history entry url"))?;
    let url = format!("{BASE_URL}/{}/{}", &caps[1], &caps[2]);
    let mal_url = MalUrl::new(name, url);

    let increment = php_int(&required_text(row, "//td[1]/strong")?);

    let date_node = required_first(row, "//td[2]")?;
    date_node.remove_child_nodes()?;
    let text = utf8_nbsp_trim(&cleanse(&date_node.node_text()));
    let date = parse_date(&text).ok_or_else(|| missing("history entry date"))?;

    Ok(json!({
        "entry": mal_url.to_json(),
        "increment": increment,
        "date": format_atom(&date),
    }))
}

fn history_url_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"/(.\w+)\.php\?id=(\d+)").expect("valid regex"))
}

// ---------------------------------------------------------------------------
// Jikan\Parser\User\ClubParser
// ---------------------------------------------------------------------------

/// `Jikan\Parser\User\ClubParser`.
pub struct ClubParser {
    node: HtmlNode,
}

impl ClubParser {
    pub fn new(doc: &HtmlDoc) -> Self {
        ClubParser { node: doc.root() }
    }

    /// `ClubParser::getClubs()`.
    pub fn get_clubs(&self) -> Result<Vec<Value>, ParseError> {
        let nodes = self
            .node
            .nodes("//*[@id=\"content\"]/table/tr/td[2]/ol/li")?;
        let mut clubs = Vec::with_capacity(nodes.len());
        for node in &nodes {
            let href = required_attr(node, "//a", "href")?;
            clubs.push(json!({
                "mal_id": club_id_from_url(&href),
                "name": required_text(node, "//a")?,
                "url": format!("{BASE_URL}{href}"),
            }));
        }
        Ok(clubs)
    }
}

// ---------------------------------------------------------------------------
// Jikan\Parser\User\UsernameByIdParser
// ---------------------------------------------------------------------------

/// `Jikan\Parser\User\UsernameByIdParser`.
pub struct UsernameByIdParser {
    node: HtmlNode,
}

impl UsernameByIdParser {
    pub fn new(doc: &HtmlDoc) -> Self {
        UsernameByIdParser { node: doc.root() }
    }

    /// `getUser()`: `UserMetaBasic` -> `{url, username}`.
    pub fn get_user(&self) -> Result<Value, ParseError> {
        let node = required_first(&self.node, "//*[@id=\"content\"]/div[1]/div[1]/a")?;
        let username = username_re()
            .captures(&node.node_text())
            .map(|caps| caps[1].to_string());
        Ok(json!({
            "url": format!("{BASE_URL}{}", node.node_attr("href").unwrap_or_default()),
            "username": username,
        }))
    }
}

fn username_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(.*?)'s Profile").expect("valid regex"))
}

// ---------------------------------------------------------------------------
// Jikan\Model\User\AnimeListItem / MangaListItem factories
// ---------------------------------------------------------------------------

fn list_item_mal_urls(item: &Value, field: &str, kind: &str, canonical: bool) -> Vec<Value> {
    let Some(Value::Array(entries)) = item.get(field) else {
        return Vec::new();
    };
    entries
        .iter()
        .filter_map(|entry| {
            let name = string_field(entry, "name")?;
            let id = entry.get("id").map(php_int_value).unwrap_or(0);
            let slug = if canonical {
                str_to_canonical(&name)
            } else {
                name.clone()
            };
            let url = format!("{BASE_URL}/{kind}/{id}/{slug}");
            Some(MalUrl::new(name, url).to_json())
        })
        .collect()
}

/// `Jikan\Model\User\AnimeListItem::factory()`.
pub fn anime_list_item(item: &Value) -> Value {
    let image = string_field(item, "anime_image_path")
        .map(|path| parse_image_quality(&path))
        .unwrap_or_default();

    let season = item.get("anime_season");
    let season_name = season
        .and_then(|season| season.get("season"))
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(Value::Null);
    let season_year = season
        .and_then(|season| season.get("year"))
        .cloned()
        .unwrap_or(Value::Null);

    let studios = list_item_mal_urls(item, "anime_studios", "anime/producer", true);
    let licensors = list_item_mal_urls(item, "anime_licensors", "anime/producer", true);
    let genres = list_item_mal_urls(item, "genres", "anime/genre", false);
    let demographics = list_item_mal_urls(item, "demographics", "anime/genre", false);

    json!({
        "mal_id": raw_field(item, "anime_id"),
        "title": raw_field(item, "anime_title"),
        "video_url": format!("{BASE_URL}{}", string_field(item, "video_url").unwrap_or_default()),
        "url": format!("{BASE_URL}{}", string_field(item, "anime_url").unwrap_or_default()),
        "images": common_image_resource(if image.is_empty() { None } else { Some(&image) }),
        "type": raw_field(item, "anime_media_type_string"),
        "watching_status": raw_field(item, "status"),
        "score": raw_field(item, "score"),
        "watched_episodes": raw_field(item, "num_watched_episodes"),
        "total_episodes": raw_field(item, "anime_num_episodes"),
        "airing_status": raw_field(item, "anime_airing_status"),
        "season_name": season_name,
        "season_year": season_year,
        "has_episode_video": optional_bool_field(item, "has_episode_video"),
        "has_promo_video": optional_bool_field(item, "has_promotion_video"),
        "has_video": optional_bool_field(item, "has_video"),
        "is_rewatching": json!(php_bool(&optional_bool_field(item, "is_rewatching"))),
        "tags": if php_empty(item.get("tags")) { Value::Null } else { raw_field(item, "tags") },
        "rating": raw_field(item, "anime_mpaa_rating_string"),
        "start_date": date_value(parse_date_dmy(string_field(item, "anime_start_date_string").as_deref())),
        "end_date": date_value(parse_date_dmy(string_field(item, "anime_end_date_string").as_deref())),
        "watch_start_date": date_value(parse_date_dmy(string_field(item, "start_date_string").as_deref())),
        "watch_end_date": date_value(parse_date_dmy(string_field(item, "finish_date_string").as_deref())),
        "days": raw_field(item, "days_string"),
        "storage": if php_empty(item.get("storage_string")) { Value::Null } else { raw_field(item, "storage_string") },
        "priority": raw_field(item, "priority_string"),
        "added_to_list": optional_bool_field(item, "is_added_to_list"),
        "studios": studios,
        "licensors": licensors,
        "genres": genres,
        "demographics": demographics,
    })
}

/// `Jikan\Model\User\MangaListItem::factory()`.
pub fn manga_list_item(item: &Value) -> Value {
    let image = string_field(item, "manga_image_path")
        .map(|path| parse_image_quality(&path))
        .unwrap_or_default();

    let magazines = list_item_mal_urls(item, "manga_magazines", "manga/magazine", true);
    let genres = list_item_mal_urls(item, "genres", "manga/genre", false);
    let demographics = list_item_mal_urls(item, "demographics", "manga/genre", false);

    json!({
        "mal_id": raw_field(item, "manga_id"),
        "title": raw_field(item, "manga_title"),
        "url": format!("{BASE_URL}{}", string_field(item, "manga_url").unwrap_or_default()),
        "images": common_image_resource(if image.is_empty() { None } else { Some(&image) }),
        "type": raw_field(item, "manga_media_type_string"),
        "reading_status": raw_field(item, "status"),
        "score": raw_field(item, "score"),
        "read_chapters": raw_field(item, "num_read_chapters"),
        "read_volumes": raw_field(item, "num_read_volumes"),
        "total_chapters": raw_field(item, "manga_num_chapters"),
        "total_volumes": raw_field(item, "manga_num_volumes"),
        "publishing_status": raw_field(item, "manga_publishing_status"),
        "is_rereading": json!(php_bool(&optional_bool_field(item, "is_rereading"))),
        "tags": if php_empty(item.get("tags")) { Value::Null } else { raw_field(item, "tags") },
        "start_date": date_value(parse_date_mdy(string_field(item, "manga_start_date_string").as_deref())),
        "end_date": date_value(parse_date_mdy(string_field(item, "manga_end_date_string").as_deref())),
        "read_start_date": date_value(parse_date_mdy(string_field(item, "start_date_string").as_deref())),
        "read_end_date": date_value(parse_date_mdy(string_field(item, "finish_date_string").as_deref())),
        "days": raw_field(item, "days_string"),
        "retail": if php_empty(item.get("retail_string")) { Value::Null } else { raw_field(item, "retail_string") },
        "priority": raw_field(item, "priority_string"),
        "added_to_list": optional_bool_field(item, "is_added_to_list"),
        "magazines": magazines,
        "genres": genres,
        "demographics": demographics,
    })
}

// ---------------------------------------------------------------------------
// Jikan\Parser\User\Reviews\UserReviewsParser
//
// The review item itself is delegated to `parser::reviews`
// (`AnimeReviewParser`/`MangaReviewParser`), which owns those ports.
// ---------------------------------------------------------------------------

/// `Jikan\Parser\User\Reviews\UserReviewsParser`.
pub struct UserReviewsParser {
    node: HtmlNode,
}

impl UserReviewsParser {
    pub fn new(doc: &HtmlDoc) -> Self {
        UserReviewsParser { node: doc.root() }
    }

    /// `UserReviews::fromParser()`: `has_next_page` is hardcoded `true` and
    /// `last_visible_page` hardcoded `1` upstream.
    pub fn get_model(&self) -> Result<Value, ParseError> {
        let nodes = self.node.nodes(
            "//*[@id=\"content\"]/table/tr/td[2]//div[contains(@class, \"review-element\")]",
        )?;
        let mut results = Vec::with_capacity(nodes.len());
        for node in &nodes {
            results.push(user_review_item(node)?);
        }
        Ok(json!({
            "results": results,
            "has_next_page": true,
            "last_visible_page": 1,
        }))
    }
}

/// `UserAnimeReview` / `UserMangaReview` JMS shape: the `Review` base fields
/// first, then the anime/manga progress field, then `entry` (declaration
/// order).
#[allow(clippy::too_many_arguments)]
fn user_review_payload(
    mal_id: i64,
    url: String,
    review_type: String,
    reactions: Value,
    date: Option<DateTime<FixedOffset>>,
    score: i64,
    review: String,
    tags: Vec<String>,
    is_spoiler: bool,
    is_preliminary: bool,
    progress_key: &str,
    progress: Option<i64>,
    entry: Value,
) -> Value {
    let mut value = json!({
        "mal_id": mal_id,
        "url": url,
        "type": review_type,
        "reactions": reactions,
        "date": date_value(date),
        "review": review,
        "score": score,
        "tags": tags,
        "is_spoiler": is_spoiler,
        "is_preliminary": is_preliminary,
    });
    if let Some(map) = value.as_object_mut() {
        map.insert(
            progress_key.to_string(),
            progress
                .map(|progress| json!(progress))
                .unwrap_or(Value::Null),
        );
        map.insert("entry".to_string(), entry);
    }
    value
}

fn user_review_item(crawler: &HtmlNode) -> Result<Value, ParseError> {
    // `UserReviewsParser::getReviews()` dispatches on the raw type marker.
    let kind_text = required_text(crawler, "//div/div/div[2]/div[2]/small")?;
    match kind_text.as_str() {
        "(Anime)" => {
            let parser = AnimeReviewParser::new(crawler);
            let entry = common_meta(
                &parser.get_anime_title()?,
                &parser.get_anime_url()?,
                &parser.get_anime_image_url_from_user_page()?,
            );
            let mal_id = parser.get_id()?;
            let url = parser.get_url()?;
            let review_type = parser.get_type()?.unwrap_or_else(|| "anime".to_string());
            let reactions = parser.get_reactions()?;
            let date = parser.get_date()?;
            let score = parser.get_reviewer_score()?;
            let review = parser.get_content()?;
            let tags = parser.get_review_tag()?;
            let is_preliminary = parser.is_preliminary()?;
            // `getReviewTag()` above detached the preliminary span, so this is
            // the PHP "episodesWatched is read after tags" quirk (null).
            let progress = parser.get_episodes_watched()?;
            let is_spoiler = parser.is_spoiler()?;
            Ok(user_review_payload(
                mal_id,
                url,
                review_type,
                reactions,
                date,
                score,
                review,
                tags,
                is_spoiler,
                is_preliminary,
                "episodes_watched",
                progress,
                entry,
            ))
        }
        "(Manga)" => {
            let parser = MangaReviewParser::new(crawler);
            let entry = common_meta(
                &parser.get_manga_title()?,
                &parser.get_manga_url()?,
                &parser.get_manga_image_url_from_user_page()?,
            );
            let mal_id = parser.get_id()?;
            let url = parser.get_url()?;
            let review_type = parser.get_type()?.unwrap_or_else(|| "manga".to_string());
            let reactions = parser.get_reactions()?;
            let date = parser.get_date()?;
            let score = parser.get_reviewer_score()?;
            let review = parser.get_content()?;
            let tags = parser.get_review_tag()?;
            let is_preliminary = parser.is_preliminary()?;
            let progress = parser.get_chapters_read()?;
            let is_spoiler = parser.is_spoiler()?;
            Ok(user_review_payload(
                mal_id,
                url,
                review_type,
                reactions,
                date,
                score,
                review,
                tags,
                is_spoiler,
                is_preliminary,
                "chapters_read",
                progress,
                entry,
            ))
        }
        _ => Err(missing("//div/div/div[2]/div[2]/small")),
    }
}

// ---------------------------------------------------------------------------
// Jikan\Parser\Recommendations\UserRecommendationsParser
//
// Item parsing is delegated to `parser::recommendations`
// (`RecommendationListItemParser`), which owns that port.
// ---------------------------------------------------------------------------

/// `Jikan\Parser\Recommendations\UserRecommendationsParser`.
pub struct UserRecommendationsParser {
    node: HtmlNode,
}

impl UserRecommendationsParser {
    pub fn new(doc: &HtmlDoc) -> Self {
        UserRecommendationsParser { node: doc.root() }
    }

    fn results(&self) -> Result<Vec<Value>, ParseError> {
        let nodes = self.node.nodes(
            "//*[@id=\"content\"]/div/div[2]/div/div[2]/div[contains(@class, \"spaceit borderClass\")]",
        )?;
        let mut results = Vec::with_capacity(nodes.len());
        for node in &nodes {
            results.push(RecommendationListItemParser::new(node).get_model()?);
        }
        Ok(results)
    }

    fn has_next_page(&self) -> Result<bool, ParseError> {
        Ok(!self
            .node
            .nodes(
                "//*[@id=\"content\"]/div/div[2]/div/div[2]/div[1]/a[contains(text(), \"More Recommendations\")]",
            )?
            .is_empty())
    }

    fn last_page(&self) -> Result<i64, ParseError> {
        let Some(node) = self.node.first(
            "//*[@id=\"content\"]/div/div[2]/div/div[2]/div[2]/div[contains(text(), \"Total Recommendations:\")]",
        )? else {
            return Ok(1);
        };
        let text = node.node_text();
        let Some(caps) = total_recommendations_re().captures(&text) else {
            return Ok(1);
        };
        let total = php_int(&caps[1].replace(',', ""));
        Ok((total as f64 / 30.0).ceil() as i64)
    }

    /// `UserRecommendations::fromParser()`.
    pub fn get_model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "results": self.results()?,
            "has_next_page": self.has_next_page()?,
            "last_visible_page": self.last_page()?,
        }))
    }
}

fn total_recommendations_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"Total Recommendations: (.*)$").expect("valid regex"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(html: &str) -> HtmlDoc {
        HtmlDoc::parse_str(html).expect("fixture html parses")
    }

    #[test]
    fn split_type_and_year_reads_favorites_label() {
        assert_eq!(
            split_type_and_year("Movie\u{B7}1995"),
            ("Movie".to_string(), 1995)
        );
        assert_eq!(
            split_type_and_year("TV\u{B7}2006"),
            ("TV".to_string(), 2006)
        );
        // no separator -> PHP notice + (int) null = 0
        assert_eq!(split_type_and_year("Movie"), ("Movie".to_string(), 0));
    }

    #[test]
    fn history_item_parses_relative_mal_url() {
        let row = doc(
            "<table><tr><td class=\"borderClass\"><a href=\"/anime.php?id=33191\">Kishibe Rohan wa Ugokanai</a> ep. <strong>4</strong></td><td class=\"borderClass\" align=\"right\">&nbsp;Sep 27, 6:50 AM</td></tr></table>",
        );
        let row = row.first("//tr").unwrap().unwrap();
        let value = history_item(&row).expect("history row");
        assert_eq!(
            value["entry"],
            json!({
                "mal_id": 33191,
                "type": "anime",
                "name": "Kishibe Rohan wa Ugokanai",
                "url": "https://myanimelist.net/anime/33191",
            })
        );
        assert_eq!(value["increment"], 4);
        assert_eq!(value["date"].as_str().unwrap().len(), 25);
    }

    #[test]
    fn username_by_id_strips_s_profile_suffix() {
        let document = doc(
            "<div id=\"content\"><div><div><a href=\"/profile/sandshark\">sandshark's Profile</a></div></div></div>",
        );
        let parser = UsernameByIdParser::new(&document);
        assert_eq!(
            parser.get_user().unwrap(),
            json!({
                "url": "https://myanimelist.net/profile/sandshark",
                "username": "sandshark",
            })
        );
    }

    #[test]
    fn user_images_mirror_php_null_handling() {
        assert_eq!(
            user_image_resource(None),
            json!({ "jpg": { "image_url": null }, "webp": { "image_url": "" } })
        );
        assert_eq!(
            common_image_resource(None),
            json!({
                "jpg": { "image_url": null, "small_image_url": null, "large_image_url": null },
                "webp": { "image_url": "", "small_image_url": "", "large_image_url": "" },
            })
        );
        assert_eq!(
            character_image_resource(None),
            json!({
                "jpg": { "image_url": null },
                "webp": { "image_url": "", "small_image_url": "" },
            })
        );
        assert_eq!(
            person_image_resource(None),
            json!({ "jpg": { "image_url": null } })
        );
    }

    #[test]
    fn php_cast_helpers() {
        assert!(php_bool(&json!(1)));
        assert!(!php_bool(&json!(0)));
        assert!(!php_bool(&json!("0")));
        assert!(php_bool(&json!("x")));
        assert!(php_empty(None));
        assert!(php_empty(Some(&json!(""))));
        assert!(!php_empty(Some(&json!(["a"]))));
        assert_eq!(php_int("  12abc"), 12);
        assert_eq!(php_int("-3"), -3);
        assert_eq!(php_int("abc"), 0);
        assert_eq!(php_int_value(&json!("12")), 12);
        assert_eq!(php_int_value(&json!(true)), 1);
    }

    #[test]
    fn review_reactions_come_from_shared_parser() {
        // `UserReviewsParser` uses `parser::reviews::ReactionsParser`; an empty
        // `data-reactions` attribute falls back to the PHP default payload.
        let document = doc("<div class=\"review-element\" data-reactions=\"\"></div>");
        let node = document.first("//div").unwrap().unwrap();
        assert_eq!(
            crate::parser::reviews::ReactionsParser::new(&node)
                .get_model()
                .unwrap(),
            json!({
                "overall": 0,
                "nice": 0,
                "love_it": 0,
                "funny": 0,
                "confusing": 0,
                "informative": 0,
                "well_written": 0,
                "creative": 0,
            })
        );
    }
}
