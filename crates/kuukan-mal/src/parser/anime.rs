//! Anime parsers: port of `Jikan\Parser\Anime\*` plus the small shared
//! `Jikan\Parser\Common` pieces those parsers pull in (character/staff list
//! items, person/user meta, images, Youtube meta, DateRange).
//!
//! Every parser mirrors the PHP class of the same name; getters return
//! `Result<T, ParseError>` (`Result<T, AnimeError>` for
//! [`AnimeEpisodeParser`], whose missing-page signal becomes a MAL 404), and
//! `get_model()` builds the JMS-shaped `serde_json::Value` inline, exactly like
//! `App\Providers\SerializerFactory` + JMS `CamelCaseNamingStrategy` would.
//!
//! Shared helpers from `parser/common.rs` are used by the API layer for
//! recommendations/pictures; the pieces this family needs locally are kept
//! private to stay independent of in-progress sibling modules.

use std::sync::OnceLock;

use chrono::{DateTime, Datelike, FixedOffset, TimeZone};
use regex::Regex;
use serde_json::{json, Map, Value};

use crate::error::ParseError;
use crate::parser::date;
use crate::parser::helper::{
    generate_youtube_url_from_id, parse_image_quality, parse_image_thumb_to_hq, xpath_literal,
    youtube_id_from_url, youtube_image_resource, HtmlDoc, HtmlNode,
};
use crate::parser::jstring::{cleanse, is_string_float, utf8_nbsp_trim};
use crate::parser::mal_url::{id_from_url, MalUrlParser};

type PResult<T> = Result<T, ParseError>;

/// Failure of [`AnimeEpisodeParser`].
///
/// MAL returns HTTP 200 for a non-existent episode page; PHP throws
/// `BadResponseException(404, 404)` from the parser, which `MalClient`
/// re-throws as `BadResponseException('404 on <path>', 404)`.
#[derive(Debug)]
pub enum AnimeError {
    /// Infrastructure/runtime failure (mirrors `ParserException::fromRequest`).
    Parse(ParseError),
    /// The h2/span marker is missing: MAL served a page that does not exist.
    EpisodeNotFound,
}

impl From<ParseError> for AnimeError {
    fn from(error: ParseError) -> Self {
        AnimeError::Parse(error)
    }
}

impl std::fmt::Display for AnimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnimeError::Parse(error) => write!(f, "{error}"),
            AnimeError::EpisodeNotFound => write!(f, "episode not found"),
        }
    }
}

impl std::error::Error for AnimeError {}

// ---------------------------------------------------------------------------
// small helpers shared by the parsers
// ---------------------------------------------------------------------------

/// PHP `empty()` for strings (`""` and `"0"` are empty).
fn php_empty(value: &str) -> bool {
    value.is_empty() || value == "0"
}

/// PHP `(int)` cast on a string: leading whitespace/sign, then digits.
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
    let digits = value[start..i].parse::<i64>().unwrap_or(i64::MAX);
    if negative {
        -digits
    } else {
        digits
    }
}

/// PHP `(float)` cast on a decimal string; unparsable values become `0.0`.
fn php_float(value: &str) -> f64 {
    let trimmed = value.trim_start_matches(|c: char| c.is_ascii_whitespace());
    let bytes = trimmed.as_bytes();
    let mut i = 0;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    let digits_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }
    if i == digits_start {
        return 0.0;
    }
    trimmed[..i].parse::<f64>().unwrap_or(0.0)
}

/// `(int) preg_replace('/\D/', '', $input)` (`AnimeStatsParser::sanitize`).
fn digit_only_int(value: &str) -> i64 {
    let digits: String = value.chars().filter(char::is_ascii_digit).collect();
    php_int(&digits)
}

/// A required attribute on the first match of `xpath`.
fn required_attr(doc: &HtmlDoc, xpath: &str, name: &str) -> PResult<String> {
    doc.attr(xpath, name)?
        .ok_or_else(|| ParseError::Html(format!("missing @{name} for xpath {xpath}")))
}

/// A required attribute on a single node.
fn node_required_attr(node: &HtmlNode, name: &str) -> PResult<String> {
    node.node_attr(name)
        .ok_or_else(|| ParseError::Html(format!("missing @{name} on {}", node.node_name())))
}

/// `$crawler->ancestors()->text()`: the text of the nearest element ancestor.
fn first_ancestor_text(node: &HtmlNode) -> String {
    node.ancestors()
        .into_iter()
        .next()
        .map(|ancestor| ancestor.node_text())
        .unwrap_or_default()
}

/// `$crawler->ancestors()->first()`.
fn first_ancestor(node: &HtmlNode) -> Option<HtmlNode> {
    node.ancestors().into_iter().next()
}

/// `(new MalUrlParser($crawler))->getModel()` serialized by JMS.
fn mal_url_value(node: &HtmlNode) -> PResult<Value> {
    Ok(MalUrlParser::new(node.clone()).get_model()?.to_json())
}

/// PHP `http_build_query`-free simple query extraction (`parse_str` + `(int)`).
fn query_param(url: &str, key: &str) -> Option<String> {
    let query = url.split_once('?')?.1;
    let query = query.split('#').next().unwrap_or(query);
    let mut found = None;
    for pair in query.split('&') {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        if k == key {
            found = Some(v.to_string());
        }
    }
    found
}

/// PHP `preg_match('~(.*)\((.*)\)~')` -> `(group1, group2)`, `empty()`-filtered.
fn paren_parts(text: &str) -> (Option<String>, Option<String>) {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"^(.*)\((.*)\)").expect("valid regex"));
    match re.captures(text) {
        Some(caps) => {
            let first = caps.get(1).map(|m| m.as_str().to_string());
            let second = caps.get(2).map(|m| m.as_str().to_string());
            (
                first.filter(|s| !php_empty(s)),
                second.filter(|s| !php_empty(s)),
            )
        }
        None => (None, None),
    }
}

// ---------------------------------------------------------------------------
// JMS-shaped common structures
// ---------------------------------------------------------------------------

/// `CommonImageResource` (`{jpg, webp}` with small/large variants).
///
/// PHP's `Webp::factory()` runs `str_replace()` on the raw value, so a `null`
/// image URL becomes `""` (PHP 8 deprecation) instead of `null`.
fn common_image(image_url: Option<&str>) -> Value {
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

/// `CharacterImageResource` (jpg + webp without the large variant).
fn character_image(image_url: Option<&str>) -> Value {
    let jpg = match image_url {
        Some(url) => json!({ "image_url": url }),
        None => json!({ "image_url": null }),
    };
    let webp = match image_url {
        Some(url) => json!({
            "image_url": url.replace(".jpg", ".webp"),
            "small_image_url": url.replace(".jpg", "t.webp"),
        }),
        None => json!({ "image_url": "", "small_image_url": "" }),
    };
    json!({ "jpg": jpg, "webp": webp })
}

/// `PersonImageResource` (jpg only).
fn person_image(image_url: Option<&str>) -> Value {
    match image_url {
        Some(url) => json!({ "jpg": { "image_url": url } }),
        None => json!({ "jpg": { "image_url": null } }),
    }
}

/// `UserImageResource` (jpg + webp, single URL each).
fn user_image(image_url: Option<&str>) -> Value {
    let jpg = match image_url {
        Some(url) => json!({ "image_url": url }),
        None => json!({ "image_url": null }),
    };
    let webp = match image_url {
        Some(url) => json!({ "image_url": url.replace(".jpg", ".webp") }),
        None => json!({ "image_url": "" }),
    };
    json!({ "jpg": jpg, "webp": webp })
}

/// `WrapImageResource` (jpg only).
fn wrap_image(image_url: Option<&str>) -> Value {
    match image_url {
        Some(url) => json!({ "jpg": { "image_url": url } }),
        None => json!({ "jpg": { "image_url": null } }),
    }
}

/// `\Jikan\Model\Common\YoutubeMeta::factory($embedUrl)`.
fn youtube_meta(embed_url: Option<&str>) -> Value {
    let youtube_id = youtube_id_from_url(embed_url);
    json!({
        "youtube_id": youtube_id,
        "url": generate_youtube_url_from_id(youtube_id.as_deref()),
        "embed_url": embed_url,
        "images": youtube_image_resource(youtube_id.as_deref()),
    })
}

/// `\Jikan\Model\Common\PersonMeta`.
fn person_meta(name: &str, url: &str, image_url: &str) -> Value {
    let image = parse_image_quality(image_url);
    json!({
        "mal_id": id_from_url(url),
        "url": url,
        "images": person_image(Some(&image)),
        "name": name,
    })
}

/// `\Jikan\Model\Common\CharacterMeta`.
fn character_meta(name: &str, url: &str, image_url: &str) -> Value {
    let image = parse_image_quality(image_url);
    json!({
        "mal_id": id_from_url(url),
        "url": url,
        "images": character_image(Some(&image)),
        "name": name,
    })
}

/// `\Jikan\Model\Common\UserMeta`.
fn user_meta(username: &str, url: &str, image_url: &str) -> Value {
    let image = parse_image_quality(image_url);
    json!({
        "username": username,
        "url": url,
        "images": user_image(Some(&image)),
    })
}

/// `\Jikan\Model\Common\MusicMeta`.
fn music_meta(title: Option<&str>, author: Option<&str>) -> Value {
    json!({ "title": title, "author": author })
}

/// `\Jikan\Model\Common\Url` (`UrlParser`).
fn url_model(node: &HtmlNode) -> PResult<Value> {
    Ok(json!({
        "name": cleanse(&node.node_text()),
        "url": cleanse(&node_required_attr(node, "href")?),
    }))
}

/// `DateProp::fromDateTime()` fields.
fn date_prop(date: Option<&DateTime<FixedOffset>>) -> Value {
    match date {
        Some(dt) => json!({
            "day": dt.day(),
            "month": dt.month(),
            "year": dt.year(),
        }),
        None => json!({ "day": null, "month": null, "year": null }),
    }
}

/// `\Jikan\Model\Common\DateRange` serialization.
fn date_range_value(date: &str) -> Value {
    let from = if date == "Not available" {
        None
    } else if date.contains(" to ") {
        date::parse_date(date.split(" to ").next().unwrap_or(""))
    } else {
        date::parse_date(date)
    };
    let until = if !date.contains(" to ") || date.contains(" to ?") {
        None
    } else {
        date::parse_date(date.split(" to ").nth(1).unwrap_or(""))
    };
    json!({
        "from": from.as_ref().map(date::format_atom),
        "to": until.as_ref().map(date::format_atom),
        "prop": {
            "from": date_prop(from.as_ref()),
            "to": date_prop(until.as_ref()),
        },
        "string": date,
    })
}

/// `(new DateRange($string))` when the string may be empty.
fn date_range_value_opt(date: &str) -> Value {
    if date.is_empty() {
        return json!({
            "from": null,
            "to": null,
            "prop": {
                "from": { "day": null, "month": null, "year": null },
                "to": { "day": null, "month": null, "year": null },
            },
            "string": "",
        });
    }
    date_range_value(date)
}

// ---------------------------------------------------------------------------
// AnimeParser
// ---------------------------------------------------------------------------

/// Port of `Jikan\Parser\Anime\AnimeParser`.
pub struct AnimeParser {
    doc: HtmlDoc,
}

impl AnimeParser {
    pub fn new(doc: HtmlDoc) -> Self {
        AnimeParser { doc }
    }

    /// `AnimeParser::getId()`.
    pub fn get_id(&self) -> PResult<i64> {
        Ok(id_from_url(&self.get_url()?))
    }

    /// `AnimeParser::getURL()`.
    pub fn get_url(&self) -> PResult<String> {
        required_attr(&self.doc, "//meta[@property='og:url']", "content")
    }

    /// `AnimeParser::getTitle()`.
    pub fn get_title(&self) -> PResult<String> {
        required_attr(&self.doc, "//meta[@property='og:title']", "content")
    }

    /// `AnimeParser::getImageURL()`.
    pub fn get_image_url(&self) -> PResult<String> {
        required_attr(&self.doc, "//meta[@property='og:image']", "content")
    }

    /// `AnimeParser::getSynopsis()`.
    pub fn get_synopsis(&self) -> PResult<Option<String>> {
        let Some(html) = self.doc.html("//p[@itemprop='description']")? else {
            return Ok(None);
        };
        let synopsis = cleanse(&html);
        if synopsis.starts_with("No synopsis information has been added to this title.") {
            Ok(None)
        } else {
            Ok(Some(synopsis))
        }
    }

    /// `AnimeParser::getApproved()`.
    pub fn get_approved(&self) -> PResult<bool> {
        let count = self
            .doc
            .count("//*[@id=\"addtolist\"]/span[contains(text(), \"pending approval\")]")?;
        Ok(count == 0)
    }

    /// `AnimeParser::getTitleEnglish()`.
    pub fn get_title_english(&self) -> PResult<Option<String>> {
        let Some(span) = self.doc.first("//span[text()=\"English:\"]")? else {
            return Ok(None);
        };
        Ok(Some(cleanse(
            &first_ancestor_text(&span).replace(&span.node_text(), ""),
        )))
    }

    /// `AnimeParser::getTitleSynonyms()`.
    pub fn get_title_synonyms(&self) -> PResult<Vec<String>> {
        let Some(span) = self.doc.first("//span[text()=\"Synonyms:\"]")? else {
            return Ok(Vec::new());
        };
        let titles = first_ancestor_text(&span).replace(&span.node_text(), "");
        Ok(titles.split(", ").map(cleanse).collect())
    }

    /// `AnimeParser::getTitleJapanese()`.
    pub fn get_title_japanese(&self) -> PResult<Option<String>> {
        let Some(span) = self.doc.first("//span[text()=\"Japanese:\"]")? else {
            return Ok(None);
        };
        Ok(Some(cleanse(
            &first_ancestor_text(&span).replace(&span.node_text(), ""),
        )))
    }

    /// `AnimeParser::getTitles()`.
    pub fn get_titles(&self) -> PResult<Vec<Value>> {
        let mut titles = vec![json!({ "type": "Default", "title": self.get_title()? })];

        let containers = self.doc.nodes(
            "//h2[text()=\"Alternative Titles\"]/following-sibling::div[following::h2[text()=\"Information\"]]",
        )?;

        for container in &containers {
            for item in container.nodes("//div[contains(@class, \"spaceit_pad\")]")? {
                let text = item.node_text();
                match text.split_once(':') {
                    Some((kind, title)) => {
                        if kind != "Synonyms" {
                            titles.push(json!({
                                "type": kind,
                                "title": cleanse(title),
                            }));
                        } else {
                            for title in title.split(", ") {
                                titles.push(json!({
                                    "type": "Synonym",
                                    "title": cleanse(title),
                                }));
                            }
                        }
                    }
                    None => titles.push(json!({ "type": text, "title": null })),
                }
            }
        }

        Ok(titles)
    }

    /// `$span->text()`-labelled value with the label removed, without trimming
    /// or cleansing (the getters apply those themselves).
    fn labelled_raw(&self, label: &str) -> PResult<Option<(String, String)>> {
        let xpath = format!("//span[text()={}]", xpath_literal(label));
        let Some(span) = self.doc.first(&xpath)? else {
            return Ok(None);
        };
        let label_text = span.node_text();
        let value = first_ancestor_text(&span).replace(&label_text, "");
        Ok(Some((label_text, value)))
    }

    /// `AnimeParser::getType()`.
    pub fn get_type(&self) -> PResult<Option<String>> {
        let Some((_, value)) = self.labelled_raw("Type:")? else {
            return Ok(None);
        };
        let value = cleanse(&value);
        if value == "Unknown" {
            Ok(None)
        } else {
            Ok(Some(value))
        }
    }

    /// `AnimeParser::getEpisodes()`.
    pub fn get_episodes(&self) -> PResult<Option<i64>> {
        let Some((label, value)) = self.labelled_raw("Episodes:")? else {
            return Ok(None);
        };
        let raw = value.replace(&label, "");
        if raw.trim() == "Unknown" {
            return Ok(None);
        }
        Ok(Some(php_int(&raw)))
    }

    /// `AnimeParser::getStatus()`.
    pub fn get_status(&self) -> PResult<Option<String>> {
        Ok(self
            .labelled_raw("Status:")?
            .map(|(_, value)| cleanse(&value)))
    }

    /// `AnimeParser::getPremiered()`.
    pub fn get_premiered(&self) -> PResult<Option<String>> {
        let Some((_, value)) = self.labelled_raw("Premiered:")? else {
            return Ok(None);
        };
        let value = cleanse(&value);
        if value == "?" {
            Ok(None)
        } else {
            Ok(Some(value))
        }
    }

    /// `AnimeParser::getBroadcast()`.
    pub fn get_broadcast(&self) -> PResult<Option<String>> {
        Ok(self
            .labelled_raw("Broadcast:")?
            .map(|(_, value)| cleanse(&value)))
    }

    /// `$span->ancestors()->first()->filterXPath('//a')` -> `MalUrl[]`, honouring
    /// the `None found` marker.
    fn labelled_mal_urls(&self, label: &str) -> PResult<Vec<Value>> {
        let xpath = format!("//span[text()={}]", xpath_literal(label));
        let Some(span) = self.doc.first(&xpath)? else {
            return Ok(Vec::new());
        };
        if first_ancestor_text(&span).contains("None found") {
            return Ok(Vec::new());
        }
        let Some(parent) = first_ancestor(&span) else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for anchor in parent.nodes("//a")? {
            out.push(mal_url_value(&anchor)?);
        }
        Ok(out)
    }

    /// `AnimeParser::getProducers()`.
    pub fn get_producers(&self) -> PResult<Vec<Value>> {
        self.labelled_mal_urls("Producers:")
    }

    /// `AnimeParser::getLicensors()`.
    pub fn get_licensors(&self) -> PResult<Vec<Value>> {
        self.labelled_mal_urls("Licensors:")
    }

    /// `AnimeParser::getStudios()`.
    pub fn get_studios(&self) -> PResult<Vec<Value>> {
        self.labelled_mal_urls("Studios:")
    }

    /// `AnimeParser::getSource()`.
    pub fn get_source(&self) -> PResult<Option<String>> {
        Ok(self
            .labelled_raw("Source:")?
            .map(|(_, value)| cleanse(&value)))
    }

    /// Shared `getGenres()`/`getExplicitGenres()` body.
    fn genres_with_message_check(&self, labels: &[&str]) -> PResult<Vec<Value>> {
        for label in labels {
            let xpath = format!("//span[text()={}]", xpath_literal(label));
            let Some(span) = self.doc.first(&xpath)? else {
                continue;
            };
            if first_ancestor_text(&span).contains("No genres have been added yet") {
                continue;
            }
            let Some(parent) = first_ancestor(&span) else {
                continue;
            };
            let mut out = Vec::new();
            for anchor in parent.nodes("//a")? {
                out.push(mal_url_value(&anchor)?);
            }
            return Ok(out);
        }
        Ok(Vec::new())
    }

    /// `AnimeParser::getGenres()`.
    pub fn get_genres(&self) -> PResult<Vec<Value>> {
        self.genres_with_message_check(&["Genres:", "Genre:"])
    }

    /// `AnimeParser::getExplicitGenres()`.
    pub fn get_explicit_genres(&self) -> PResult<Vec<Value>> {
        self.genres_with_message_check(&["Explicit Genres:", "Explicit Genre:"])
    }

    /// `AnimeParser::getDemographics()`.
    pub fn get_demographics(&self) -> PResult<Vec<Value>> {
        self.genres_with_message_check(&["Demographic:", "Demographics:"])
    }

    /// `AnimeParser::getThemes()`.
    pub fn get_themes(&self) -> PResult<Vec<Value>> {
        self.genres_with_message_check(&["Theme:", "Themes:"])
    }

    /// `AnimeParser::getDuration()`.
    pub fn get_duration(&self) -> PResult<Option<String>> {
        let Some((label, value)) = self.labelled_raw("Duration:")? else {
            return Ok(None);
        };
        Ok(Some(cleanse(&value.replace(&label, "").replace('.', ""))))
    }

    /// `AnimeParser::getRating()`.
    pub fn get_rating(&self) -> PResult<Option<String>> {
        let Some((_, value)) = self.labelled_raw("Rating:")? else {
            return Ok(None);
        };
        let value = cleanse(&value);
        if value == "None" {
            Ok(None)
        } else {
            Ok(Some(value))
        }
    }

    /// `AnimeParser::getScore()`.
    pub fn get_score(&self) -> PResult<Option<f64>> {
        let Some(node) = self.doc.first("//span[@itemprop=\"ratingValue\"]")? else {
            return Ok(None);
        };
        let score = cleanse(&node.node_text());
        if score == "N/A" {
            return Ok(None);
        }
        Ok(Some(php_float(&score)))
    }

    /// `AnimeParser::getScoredBy()`.
    pub fn get_scored_by(&self) -> PResult<Option<i64>> {
        let Some(node) = self.doc.first("//span[@itemprop=\"ratingCount\"]")? else {
            return Ok(None);
        };
        let cleaned = cleanse(&node.node_text())
            .replace(',', "")
            .replace(" users", "")
            .replace(" user", "");
        if cleaned.trim().parse::<f64>().is_err() {
            return Ok(None);
        }
        Ok(Some(php_int(&cleaned)))
    }

    /// `AnimeParser::getRank()`.
    pub fn get_rank(&self) -> PResult<Option<i64>> {
        let Some(span) = self.doc.first("//span[text()=\"Ranked:\"]")? else {
            return Ok(None);
        };
        let Some(ancestor) = first_ancestor(&span) else {
            return Ok(None);
        };
        ancestor.remove_child_nodes()?;
        let ranked = cleanse(&ancestor.node_text());
        if ranked == "N/A" {
            return Ok(None);
        }
        Ok(Some(php_int(&ranked.replace('#', ""))))
    }

    /// `AnimeParser::getPopularity()`.
    pub fn get_popularity(&self) -> PResult<Option<i64>> {
        let Some((label, value)) = self.labelled_raw("Popularity:")? else {
            return Ok(None);
        };
        Ok(Some(php_int(&cleanse(
            &value.replace(&label, "").replace('#', ""),
        ))))
    }

    /// `AnimeParser::getMembers()`.
    pub fn get_members(&self) -> PResult<Option<i64>> {
        let Some((label, value)) = self.labelled_raw("Members:")? else {
            return Ok(None);
        };
        Ok(Some(php_int(&cleanse(
            &value.replace(&label, "").replace(',', ""),
        ))))
    }

    /// `AnimeParser::getFavorites()`.
    pub fn get_favorites(&self) -> PResult<Option<i64>> {
        let Some((label, value)) = self.labelled_raw("Favorites:")? else {
            return Ok(None);
        };
        Ok(Some(php_int(&cleanse(
            &value.replace(&label, "").replace(',', ""),
        ))))
    }

    /// `AnimeParser::getExternalLinks()`.
    pub fn get_external_links(&self) -> PResult<Vec<Value>> {
        let xpath = "//*[@id=\"content\"]/table//div[contains(@class, \"external_links\")]//a[contains(@class, \"link\") and not(contains(@class, \"js-more-links\"))]";
        let mut out = Vec::new();
        for anchor in self.doc.nodes(xpath)? {
            out.push(url_model(&anchor)?);
        }
        Ok(out)
    }

    /// `AnimeParser::getStreamingLinks()`.
    pub fn get_streaming_links(&self) -> PResult<Vec<Value>> {
        let xpath = "//*[@id=\"content\"]/table/tr/td[1]/div/div[contains(@class, \"broadcast\")]//div[contains(@class, \"broadcast\")]";
        let links = self.doc.nodes(xpath)?;
        if links.is_empty() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for node in &links {
            for anchor in node.nodes("//a")? {
                out.push(url_model(&anchor)?);
            }
        }
        Ok(out)
    }

    /// `AnimeParser::getRelated()`.
    pub fn get_related(&self) -> PResult<Value> {
        let mut related: Vec<(String, Value)> = Vec::new();

        let tiles = self.doc.nodes(
            "//div[contains(@class, \"related-entries\")]/div[contains(@class, \"entries-tile\")]/div[contains(@class, \"entry\")]",
        )?;
        for tile in tiles {
            let Some(relation_node) =
                tile.first("//div[@class=\"content\"]/div[@class=\"relation\"]")?
            else {
                continue;
            };
            let relation = cleanse(
                related_type_re()
                    .replace(&relation_node.node_text(), "")
                    .as_ref(),
            );
            let links = tile.nodes("//div[@class=\"content\"]/div[@class=\"title\"]/a")?;

            if links.len() == 1 && php_empty(&links[0].node_text()) {
                related_set(&mut related, &relation, json!([]));
                continue;
            }

            for link in &links {
                if php_empty(&link.node_text_raw()) {
                    unsafe {
                        libxml::bindings::xmlUnlinkNode(link.node().node_ptr());
                    }
                }
            }

            if let Some(first) = links.first() {
                let entry = mal_url_value(first)?;
                related_append(&mut related, &relation, entry);
            }
        }

        let rows = self
            .doc
            .nodes("//table[contains(@class, \"entries-table\")]/tr")?;
        for row in rows {
            let links = row.nodes("//td[2]//a")?;
            let Some(relation_node) = row.first("//td[1]")? else {
                continue;
            };
            let relation = cleanse(&relation_node.node_text().replace(':', ""));

            if links.len() == 1 && php_empty(&links[0].node_text()) {
                related_set(&mut related, &relation, json!([]));
                continue;
            }

            for link in &links {
                if php_empty(&link.node_text_raw()) {
                    unsafe {
                        libxml::bindings::xmlUnlinkNode(link.node().node_ptr());
                    }
                }
            }

            let mut entries = Vec::new();
            for link in &links {
                entries.push(mal_url_value(link)?);
            }
            related_set(&mut related, &relation, Value::Array(entries));
        }

        if related.is_empty() {
            Ok(json!([]))
        } else {
            let mut map = Map::new();
            for (key, value) in related {
                map.insert(key, value);
            }
            Ok(Value::Object(map))
        }
    }

    /// `AnimeParser::getBackground()`.
    pub fn get_background(&self) -> PResult<Option<String>> {
        let Some(parent) = self.doc.first("//p[@itemprop=\"description\"]/..")? else {
            return Ok(None);
        };
        parent.remove_child_nodes()?;
        for paragraph in parent.css_nodes("p")? {
            unsafe {
                libxml::bindings::xmlUnlinkNode(paragraph.node().node_ptr());
            }
        }
        let background = parent.node_text();
        if background.contains("No background information has been added to this title") {
            return Ok(None);
        }
        Ok(Some(cleanse(&background)))
    }

    /// `AnimeParser::getOpeningThemes()`.
    pub fn get_opening_themes(&self) -> PResult<Vec<String>> {
        self.theme_songs(
            "//div[@class=\"theme-songs js-theme-songs opnening\"]/table/tr",
            "No opening themes",
        )
    }

    /// `AnimeParser::getEndingThemes()`.
    pub fn get_ending_themes(&self) -> PResult<Vec<String>> {
        self.theme_songs(
            "//div[@class=\"theme-songs js-theme-songs ending\"]/table/tr",
            "No ending themes",
        )
    }

    fn theme_songs(&self, xpath: &str, marker: &str) -> PResult<Vec<String>> {
        let nodes = self.doc.nodes(xpath)?;
        if nodes.is_empty() {
            // PHP would throw on `text()` of an empty crawler; MAL always
            // renders the theme container, so an empty list is the safe port.
            return Ok(Vec::new());
        }
        if nodes[0].node_text().contains(marker) {
            return Ok(Vec::new());
        }
        Ok(nodes.iter().map(|n| cleanse(&n.node_text())).collect())
    }

    /// `AnimeParser::getAired()`.
    pub fn get_aired(&self) -> PResult<Value> {
        Ok(date_range_value_opt(&self.get_anime_aired_string()?))
    }

    /// `AnimeParser::getAnimeAiredString()`.
    pub fn get_anime_aired_string(&self) -> PResult<String> {
        let Some(html) = self.doc.html("//span[contains(text(), \"Aired\")]/..")? else {
            return Ok(String::new());
        };
        let trimmed = html.trim();
        Ok(trimmed
            .split('\n')
            .nth(1)
            .map(|line| line.trim().to_string())
            .unwrap_or_default())
    }

    /// `AnimeParser::getPreview()`.
    pub fn get_preview(&self) -> PResult<Option<String>> {
        let Some(node) = self
            .doc
            .first("//div[contains(@class, \"video-promotion\")]/a")?
        else {
            return Ok(None);
        };
        Ok(node.node_attr("href"))
    }

    /// `Anime::fromParser($parser)` serialized by JMS.
    ///
    /// The evaluation order matches `Anime::fromParser()` because
    /// `getBackground()` is destructive: `Parser::removeChildNodes()` deletes
    /// every element child of the description's parent, so `background` must be
    /// read last (PHP does exactly that).
    pub fn get_model(&self) -> PResult<Value> {
        let trailer = youtube_meta(self.get_preview()?.as_deref());
        let title = self.get_title()?;
        let url = self.get_url()?;
        let mal_id = self.get_id()?;
        let approved = self.get_approved()?;
        let images = common_image(Some(&self.get_image_url()?));
        let synopsis = self.get_synopsis()?;
        let title_english = self.get_title_english()?;
        let title_synonyms = self.get_title_synonyms()?;
        let title_japanese = self.get_title_japanese()?;
        let titles = self.get_titles()?;
        let type_ = self.get_type()?;
        let episodes = self.get_episodes()?;
        let status = self.get_status()?;
        let airing = status.as_deref() == Some("Currently Airing");
        let aired = self.get_aired()?;
        let premiered = self.get_premiered()?;
        let broadcast = self.get_broadcast()?;
        let producers = self.get_producers()?;
        let licensors = self.get_licensors()?;
        let studios = self.get_studios()?;
        let source = self.get_source()?;
        let genres = self.get_genres()?;
        let explicit_genres = self.get_explicit_genres()?;
        let demographics = self.get_demographics()?;
        let themes = self.get_themes()?;
        let duration = self.get_duration()?;
        let rating = self.get_rating()?;
        let score = self.get_score()?;
        let scored_by = self.get_scored_by()?;
        let rank = self.get_rank()?;
        let popularity = self.get_popularity()?;
        let members = self.get_members()?;
        let favorites = self.get_favorites()?;
        let external_links = self.get_external_links()?;
        let streaming_links = self.get_streaming_links()?;
        let related = self.get_related()?;
        let opening_themes = self.get_opening_themes()?;
        let ending_themes = self.get_ending_themes()?;
        // Must be last: it removes most of the page's DOM nodes.
        let background = self.get_background()?;

        let mut model = Map::new();
        model.insert("mal_id".to_string(), json!(mal_id));
        model.insert("url".to_string(), json!(url));
        model.insert("images".to_string(), images);
        model.insert("trailer".to_string(), trailer);
        model.insert("title".to_string(), json!(title));
        model.insert("title_english".to_string(), json!(title_english));
        model.insert("title_japanese".to_string(), json!(title_japanese));
        model.insert("title_synonyms".to_string(), json!(title_synonyms));
        model.insert("titles".to_string(), Value::Array(titles));
        model.insert("approved".to_string(), json!(approved));
        model.insert("type".to_string(), json!(type_));
        model.insert("source".to_string(), json!(source));
        model.insert("episodes".to_string(), json!(episodes));
        model.insert("status".to_string(), json!(status));
        model.insert("airing".to_string(), json!(airing));
        model.insert("aired".to_string(), aired);
        model.insert("duration".to_string(), json!(duration));
        model.insert("rating".to_string(), json!(rating));
        model.insert("score".to_string(), json!(score));
        model.insert("scored_by".to_string(), json!(scored_by));
        model.insert("rank".to_string(), json!(rank));
        model.insert("popularity".to_string(), json!(popularity));
        model.insert("members".to_string(), json!(members));
        model.insert("favorites".to_string(), json!(favorites));
        model.insert("synopsis".to_string(), json!(synopsis));
        model.insert("background".to_string(), json!(background));
        model.insert("premiered".to_string(), json!(premiered));
        model.insert("broadcast".to_string(), json!(broadcast));
        model.insert("related".to_string(), related);
        model.insert("producers".to_string(), Value::Array(producers));
        model.insert("licensors".to_string(), Value::Array(licensors));
        model.insert("studios".to_string(), Value::Array(studios));
        model.insert("genres".to_string(), Value::Array(genres));
        model.insert("explicit_genres".to_string(), Value::Array(explicit_genres));
        model.insert("demographics".to_string(), Value::Array(demographics));
        model.insert("themes".to_string(), Value::Array(themes));
        model.insert("opening_themes".to_string(), json!(opening_themes));
        model.insert("ending_themes".to_string(), json!(ending_themes));
        model.insert("external_links".to_string(), Value::Array(external_links));
        model.insert("streaming_links".to_string(), Value::Array(streaming_links));
        Ok(Value::Object(model))
    }
}

fn related_type_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\s\(.*\)").expect("valid regex"))
}

fn related_append(entries: &mut Vec<(String, Value)>, key: &str, value: Value) {
    if let Some((_, Value::Array(items))) = entries.iter_mut().find(|(k, _)| k == key) {
        items.push(value);
        return;
    }
    entries.push((key.to_string(), Value::Array(vec![value])));
}

fn related_set(entries: &mut Vec<(String, Value)>, key: &str, value: Value) {
    if let Some((_, existing)) = entries.iter_mut().find(|(k, _)| k == key) {
        *existing = value;
    } else {
        entries.push((key.to_string(), value));
    }
}

// ---------------------------------------------------------------------------
// EpisodesParser / EpisodeListItemParser
// ---------------------------------------------------------------------------

/// Port of `Jikan\Parser\Anime\EpisodesParser`.
pub struct EpisodesParser {
    doc: HtmlDoc,
}

impl EpisodesParser {
    pub fn new(doc: HtmlDoc) -> Self {
        EpisodesParser { doc }
    }

    /// `EpisodesParser::getEpisodes()`.
    pub fn get_episodes(&self) -> PResult<Vec<Value>> {
        let rows = self
            .doc
            .nodes("//table[contains(@class, 'js-watch-episode-list')]/tbody//tr")?;
        let mut out = Vec::new();
        for row in &rows {
            out.push(EpisodeListItemParser::new(row.clone()).get_model()?);
        }
        Ok(out)
    }

    /// `EpisodesParser::getLastPage()`.
    pub fn get_last_page(&self) -> PResult<i64> {
        let xpath = "//*[@id=\"content\"]/table/tr/td[2]/div[2]/div[2]/div[2]/div//a[contains(@class, \"link\")]";
        let pages = self.doc.nodes(xpath)?;
        let Some(last) = pages.last() else {
            return Ok(1);
        };
        let Some(href) = last.node_attr("href") else {
            return Ok(1);
        };
        match offset_re().captures(&href) {
            Some(caps) => {
                let offset = php_int(&caps[1]);
                Ok(offset / 100 + 1)
            }
            None => Ok(1),
        }
    }

    /// `EpisodesParser::getHasNextPage()`.
    pub fn get_has_next_page(&self) -> PResult<bool> {
        let beyond = self
            .doc
            .nodes("//*[@id=\"content\"]/table/tr/td[2]/div/div[2]/table/tbody/tr/td/div[2]")?;
        if let Some(node) = beyond.first() {
            if node
                .node_text()
                .contains("No episode information has been added to this title")
            {
                return Ok(false);
            }
        }

        let page_links = self
            .doc
            .nodes("//*[@id=\"content\"]/table/tr/td[2]/div[2]/div[2]/div[2]/div//a[contains(@class, \"link\")]")?;
        if page_links.is_empty() {
            return Ok(false);
        }

        let is_last = self.doc.count(
            "//*[@id=\"content\"]/table/tr/td[2]/div[2]/div[2]/div[2]/div//a[contains(@class, \"current\") and position() = last()]",
        )?;
        Ok(is_last == 0)
    }

    /// `Episodes::fromParser($parser)` serialized by JMS.
    pub fn get_model(&self) -> PResult<Value> {
        Ok(json!({
            "results": self.get_episodes()?,
            "has_next_page": self.get_has_next_page()?,
            "last_visible_page": self.get_last_page()?,
        }))
    }
}

fn offset_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\?offset=(\d+)$").expect("valid regex"))
}

/// Port of `Jikan\Parser\Anime\EpisodeListItemParser`.
pub struct EpisodeListItemParser {
    node: HtmlNode,
}

impl EpisodeListItemParser {
    pub fn new(node: HtmlNode) -> Self {
        EpisodeListItemParser { node }
    }

    /// `EpisodeListItemParser::getEpisodeId()`.
    pub fn get_episode_id(&self) -> PResult<i64> {
        let text = required_text_node(&self.node, "//td[contains(@class, 'episode-number')]")?;
        Ok(php_int(&text))
    }

    /// `EpisodeListItemParser::getEpisodeUrl()`.
    pub fn get_episode_url(&self) -> PResult<String> {
        required_node_attr(
            &self.node,
            "//td[contains(@class,\"episode-title\")]/a",
            "href",
        )
    }

    /// `EpisodeListItemParser::getTitle()`.
    pub fn get_title(&self) -> PResult<String> {
        required_text_node(&self.node, "//td[contains(@class, \"episode-title\")]/a")
    }

    /// `EpisodeListItemParser::getTitleJapanese()`.
    pub fn get_title_japanese(&self) -> PResult<Option<String>> {
        let text = first_text_node(
            &self.node,
            "//td[contains(@class, \"episode-title\")]/span[@class='di-ib']",
        )?;
        match text {
            Some(text) if !php_empty(&text) => Ok(paren_parts(&text).1),
            _ => Ok(None),
        }
    }

    /// `EpisodeListItemParser::getTitleRomanji()`.
    pub fn get_title_romanji(&self) -> PResult<Option<String>> {
        let text = first_text_node(
            &self.node,
            "//td[contains(@class, \"episode-title\")]/span[@class='di-ib']",
        )?;
        match text {
            Some(text) if !php_empty(&text) => Ok(paren_parts(&text).0),
            _ => Ok(None),
        }
    }

    /// `EpisodeListItemParser::getAired()`.
    pub fn get_aired(&self) -> PResult<Option<DateTime<FixedOffset>>> {
        let text = required_text_node(&self.node, "//td[contains(@class, 'episode-aired')]")?;
        if text == "N/A" {
            return Ok(None);
        }
        Ok(date::parse_date_mdy_readable(&text))
    }

    /// `EpisodeListItemParser::getScore()`.
    pub fn get_score(&self) -> PResult<Option<f64>> {
        let node = self
            .node
            .first("//td[contains(@class, 'episode-poll')]/@data-raw")?;
        let Some(node) = node else {
            return Ok(None);
        };
        let score = node.node_text();
        if !is_string_float(&score) {
            return Ok(None);
        }
        Ok(Some(php_float(&score)))
    }

    /// `EpisodeListItemParser::getFiller()`.
    pub fn get_filler(&self) -> PResult<bool> {
        let count = self.node.count(
            "//td[contains(@class,\"episode-title\")]/span[contains(@class, 'icon-episode-type-bg') and contains(text(), 'Filler')]",
        )?;
        Ok(count > 0)
    }

    /// `EpisodeListItemParser::getRecap()`.
    pub fn get_recap(&self) -> PResult<bool> {
        let count = self.node.count(
            "//td[contains(@class,\"episode-title\")]/span[contains(@class, 'icon-episode-type-bg') and contains(text(), 'Recap')]",
        )?;
        Ok(count > 0)
    }

    /// `EpisodeListItemParser::getVideoUrl()`.
    pub fn get_video_url(&self) -> PResult<Option<String>> {
        Ok(self
            .node
            .first("//td[contains(@class, 'episode-video')]/a")?
            .and_then(|node| node.node_attr("href")))
    }

    /// `EpisodeListItemParser::getForumUrl()`.
    pub fn get_forum_url(&self) -> PResult<Option<String>> {
        Ok(self
            .node
            .first("//td[contains(@class, 'episode-forum')]/a")?
            .and_then(|node| node.node_attr("href")))
    }

    /// `EpisodeListItem::fromParser($parser)` serialized by JMS.
    pub fn get_model(&self) -> PResult<Value> {
        Ok(json!({
            "mal_id": self.get_episode_id()?,
            "url": self.get_video_url()?,
            "title": self.get_title()?,
            "title_japanese": self.get_title_japanese()?,
            "title_romanji": self.get_title_romanji()?,
            "aired": self.get_aired()?.as_ref().map(date::format_atom),
            "score": self.get_score()?,
            "filler": self.get_filler()?,
            "recap": self.get_recap()?,
            "forum_url": self.get_forum_url()?,
        }))
    }
}

fn required_text_node(node: &HtmlNode, xpath: &str) -> PResult<String> {
    first_text_node(node, xpath)?.ok_or_else(|| ParseError::Html(format!("missing node {xpath}")))
}

fn first_text_node(node: &HtmlNode, xpath: &str) -> PResult<Option<String>> {
    Ok(node.first(xpath)?.map(|n| n.node_text()))
}

fn required_node_attr(node: &HtmlNode, xpath: &str, attr: &str) -> PResult<String> {
    node.attr(xpath, attr)?
        .ok_or_else(|| ParseError::Html(format!("missing @{attr} for {xpath}")))
}

// ---------------------------------------------------------------------------
// AnimeEpisodeParser
// ---------------------------------------------------------------------------

/// Port of `Jikan\Parser\Anime\AnimeEpisodeParser`.
pub struct AnimeEpisodeParser {
    doc: HtmlDoc,
}

impl AnimeEpisodeParser {
    pub fn new(doc: HtmlDoc) -> Self {
        AnimeEpisodeParser { doc }
    }

    /// `AnimeEpisodeParser::getEpisodeId()`.
    pub fn get_episode_id(&self) -> Result<i64, AnimeError> {
        let Some(node) = self.doc.first("//h2[contains(@class, 'fs18')]/span")? else {
            // MAL returns HTTP 200 for a page that doesn't exist, so we fail
            // the parsing here in order to send a mock 404.
            return Err(AnimeError::EpisodeNotFound);
        };
        Ok(php_int(&node.node_text().trim().replace(['-', '#'], "")))
    }

    /// `AnimeEpisodeParser::getEpisodeUrl()`.
    pub fn get_episode_url(&self) -> PResult<String> {
        required_attr(&self.doc, "//meta[@property='og:url']", "content")
    }

    /// `AnimeEpisodeParser::getTitle()`.
    pub fn get_title(&self) -> PResult<String> {
        let Some(node) = self.doc.first("//h2[contains(@class, 'fs18')]")? else {
            return Ok(String::new());
        };
        node.remove_child_nodes()?;
        Ok(cleanse(&node.node_text()))
    }

    /// `AnimeEpisodeParser::getTitleJapanese()`.
    pub fn get_title_japanese(&self) -> PResult<Option<String>> {
        let Some(node) = self.doc.first("//p[contains(@class, 'fn-grey2')]")? else {
            return Ok(None);
        };
        Ok(paren_parts(&node.node_text())
            .1
            .map(|value| cleanse(&value)))
    }

    /// `AnimeEpisodeParser::getTitleRomanji()`.
    pub fn get_title_romanji(&self) -> PResult<Option<String>> {
        let Some(node) = self.doc.first("//p[contains(@class, 'fn-grey2')]")? else {
            return Ok(None);
        };
        Ok(paren_parts(&node.node_text())
            .0
            .map(|value| cleanse(&value)))
    }

    /// `AnimeEpisodeParser::getAired()`.
    pub fn get_aired(&self) -> PResult<Option<DateTime<FixedOffset>>> {
        let Some(node) = self
            .doc
            .first("//div[contains(@class, 'di-tc pt4 pb4 pl8 pr8 ar fn-grey2')]")?
        else {
            return Ok(None);
        };
        match aired_re().captures(&node.node_text()) {
            Some(caps) => Ok(parse_date_with_timezone(&caps[1])),
            None => Ok(None),
        }
    }

    /// `AnimeEpisodeParser::getFiller()`.
    pub fn get_filler(&self) -> PResult<bool> {
        let Some(node) = self
            .doc
            .first("//span[contains(@class, 'icon-episode-type-bg')]")?
        else {
            return Ok(false);
        };
        Ok(node.node_text().trim() == "Filler")
    }

    /// `AnimeEpisodeParser::getRecap()`.
    pub fn get_recap(&self) -> PResult<bool> {
        let Some(node) = self
            .doc
            .first("//span[contains(@class, 'icon-episode-type-bg')]")?
        else {
            return Ok(false);
        };
        Ok(node.node_text().trim() == "Recap")
    }

    /// `AnimeEpisodeParser::getForumUrl()`.
    pub fn get_forum_url(&self) -> PResult<Option<String>> {
        Ok(self
            .doc
            .first("//td[contains(@class, 'episode-forum')]/a")?
            .and_then(|node| node.node_attr("href")))
    }

    /// `AnimeEpisodeParser::getSynopsis()`.
    pub fn get_synopsis(&self) -> PResult<Option<String>> {
        let Some(node) = self.doc.first("//meta[@property='og:description']")? else {
            return Ok(None);
        };
        let synopsis = node.node_attr("content");
        match &synopsis {
            Some(value) if value.starts_with("Looking for episode specific information") => {
                Ok(None)
            }
            _ => Ok(synopsis),
        }
    }

    /// `AnimeEpisodeParser::getDuration()`.
    pub fn get_duration(&self) -> PResult<Option<i64>> {
        let Some(node) = self
            .doc
            .first("//div[contains(@class, 'di-tc pt4 pb4 pl8 pr8 ar fn-grey2')]")?
        else {
            return Ok(None);
        };
        match duration_re().captures(&node.node_text()) {
            Some(caps) => Ok(date::parse_duration_to_seconds(&caps[1])),
            None => Ok(None),
        }
    }

    /// `AnimeEpisode::fromParser($parser)` serialized by JMS.
    pub fn get_model(&self) -> Result<Value, AnimeError> {
        let mal_id = self.get_episode_id()?;
        Ok(json!({
            "mal_id": mal_id,
            "url": self.get_episode_url()?,
            "title": self.get_title()?,
            "title_japanese": self.get_title_japanese()?,
            "title_romanji": self.get_title_romanji()?,
            "duration": self.get_duration()?,
            "aired": self.get_aired()?.as_ref().map(date::format_atom),
            "filler": self.get_filler()?,
            "recap": self.get_recap()?,
            "synopsis": self.get_synopsis()?,
        }))
    }
}

fn aired_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"Aired: (.*?)$").expect("valid regex"))
}

fn duration_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"Duration: (.*?)Aired").expect("valid regex"))
}

/// `Parser::parseDate()` plus the trailing timezone abbreviation MAL puts in
/// episode pages ("Oct 20, 1999(JST)"): PHP's date parser understands it, the
/// Rust port of `Parser::parseDate` does not.
fn parse_date_with_timezone(value: &str) -> Option<DateTime<FixedOffset>> {
    if let Some(dt) = date::parse_date(value) {
        return Some(dt);
    }
    let (date_part, offset) = match value.rsplit_once('(') {
        Some((date, zone)) if zone.ends_with(')') => {
            let zone = &zone[..zone.len() - 1];
            (date, timezone_offset(zone))
        }
        _ => return None,
    };
    let offset = offset?;
    let parsed = date::parse_date(date_part)?;
    // `new \DateTimeImmutable("Oct 20, 1999(JST)")` is local midnight in +09:00,
    // not the UTC instant converted to +09:00.
    offset.from_local_datetime(&parsed.naive_local()).single()
}

/// Offset of the PHP timezone abbreviations MAL uses in `Aired:` strings.
fn timezone_offset(zone: &str) -> Option<FixedOffset> {
    let hours = match zone {
        "JST" => 9,
        "UTC" | "GMT" => 0,
        "EST" => -5,
        "EDT" => -4,
        "CST" => -6,
        "CDT" => -5,
        "MST" => -7,
        "MDT" => -6,
        "PST" => -8,
        "PDT" => -7,
        "CET" => 1,
        "CEST" => 2,
        _ => return None,
    };
    FixedOffset::east_opt(hours * 3600)
}

// ---------------------------------------------------------------------------
// VideosParser and list items
// ---------------------------------------------------------------------------

/// Port of `Jikan\Parser\Anime\VideosParser`.
pub struct VideosParser {
    doc: HtmlDoc,
}

impl VideosParser {
    pub fn new(doc: HtmlDoc) -> Self {
        VideosParser { doc }
    }

    /// `VideosParser::getEpisodes()`.
    pub fn get_episodes(&self) -> PResult<Vec<Value>> {
        let nodes = self.doc.nodes(
            "//*[@id=\"content\"]/table/tr/td[2]/div[2]/div[2]/div[contains(@class, \"video-block episode-video\")]//*[contains(@class, \"video-list-outer\")]",
        )?;
        let mut out = Vec::new();
        for node in &nodes {
            out.push(StreamEpisodeListItemParser::new(node.clone()).get_model()?);
        }
        Ok(out)
    }

    /// `VideosParser::getPromos()`.
    pub fn get_promos(&self) -> PResult<Vec<Value>> {
        let nodes = self
            .doc
            .nodes("//div[contains(@class, \"video-block promotional-video\")]/section/div")?;
        let mut out = Vec::new();
        for node in &nodes {
            out.push(PromoListItemParser::new(node.clone()).get_model()?);
        }
        Ok(out)
    }

    /// `VideosParser::getMusic()`.
    pub fn get_music(&self) -> PResult<Vec<Value>> {
        let nodes = self
            .doc
            .nodes("//div[contains(@class, \"video-block music-video\")]/section/div")?;
        let mut out = Vec::new();
        for node in &nodes {
            out.push(MusicVideoListItemParser::new(node.clone()).get_model()?);
        }
        Ok(out)
    }

    /// `VideosParser::getHasNextPage()`.
    pub fn get_has_next_page(&self) -> PResult<bool> {
        let count = self.doc.count(
            "//div[contains(@class, \"video-block episode-video\")]//div[contains(@class, \"pagination\")]/a[text()[contains(.,\"More\")]]",
        )?;
        Ok(count > 0)
    }

    /// `VideosParser::getLastPage()`.
    pub fn get_last_page(&self) -> PResult<i64> {
        let pagination = "//div[contains(@class, \"video-block episode-video\")]//div[contains(@class, \"pagination\")]";

        // All pages except the last page returns "Last" button.
        let last = self
            .doc
            .first(&format!("{pagination}/a[text()[contains(.,\"Last\")]]"))?;
        if let Some(node) = last {
            let page = node
                .node_attr("href")
                .and_then(|href| query_param(&href, "p"))
                .map(|value| php_int(&value))
                .unwrap_or(0);
            return Ok(page);
        }

        // Fallback 1: the second last page only returns "More".
        let more = self
            .doc
            .first(&format!("{pagination}/a[text()[contains(.,\"More\")]]"))?;
        if let Some(node) = more {
            let page = node
                .node_attr("href")
                .and_then(|href| query_param(&href, "p"))
                .map(|value| php_int(&value))
                .unwrap_or(0);
            return Ok(page);
        }

        // Fallback 2: the last page is a non-clickable span.
        let node = self
            .doc
            .first(&format!("{pagination}/*[position()=last()]"))?;

        // Fallback 3: pagination shown although the user exceeded it.
        let has_reached_the_end = self.doc.count(
            "//div[contains(@class, \"video-block episode-video\")]//p[text()[contains(.,\"No episode video has been added to this title\")]]",
        )?;
        if has_reached_the_end > 0 {
            return Ok(1);
        }

        if let Some(node) = node {
            return Ok(php_int(&node.node_text()));
        }

        Ok(1)
    }

    /// `AnimeVideos::fromParser($parser)` serialized by JMS.
    pub fn get_model(&self) -> PResult<Value> {
        Ok(json!({
            "promo": self.get_promos()?,
            "episodes": self.get_episodes()?,
            "music_videos": self.get_music()?,
        }))
    }

    /// `AnimeVideosEpisodes::fromParser($parser)` serialized by JMS.
    pub fn get_results_model(&self) -> PResult<Value> {
        Ok(json!({
            "results": self.get_episodes()?,
            "has_next_page": self.get_has_next_page()?,
            "last_visible_page": self.get_last_page()?,
        }))
    }
}

/// Port of `Jikan\Parser\Anime\StreamEpisodeListItemParser`.
pub struct StreamEpisodeListItemParser {
    node: HtmlNode,
}

impl StreamEpisodeListItemParser {
    pub fn new(node: HtmlNode) -> Self {
        StreamEpisodeListItemParser { node }
    }

    /// `StreamEpisodeListItemParser::getMalId()`.
    pub fn get_mal_id(&self) -> PResult<Option<i64>> {
        let url = self.get_url()?;
        Ok(suffix_digits_re()
            .captures(&url)
            .map(|caps| php_int(&caps[1])))
    }

    /// `StreamEpisodeListItemParser::getTitle()`.
    pub fn get_title(&self) -> PResult<String> {
        required_text_node(&self.node, "//a/div/span/span[@class=\"episode-title\"]")
    }

    /// `StreamEpisodeListItemParser::getEpisode()`.
    pub fn get_episode(&self) -> PResult<String> {
        let Some(node) = self
            .node
            .first("//a/div/span[contains(@class,\"title\")]")?
        else {
            return Ok(String::new());
        };
        node.remove_child_nodes()?;
        Ok(node.node_text())
    }

    /// `StreamEpisodeListItemParser::getUrl()`.
    pub fn get_url(&self) -> PResult<String> {
        required_node_attr(&self.node, "//a", "href")
    }

    /// `StreamEpisodeListItemParser::getImageUrl()`.
    pub fn get_image_url(&self) -> PResult<Option<String>> {
        let image = self.node.attr("//a/img", "data-src")?;
        if image.as_deref() == Some("https://cdn.myanimelist.net/images/icon-banned-youtube.png") {
            return Ok(None);
        }
        Ok(image)
    }

    /// `StreamEpisodeListItem::fromParser($parser)` serialized by JMS.
    pub fn get_model(&self) -> PResult<Value> {
        Ok(json!({
            "mal_id": self.get_mal_id()?,
            "title": self.get_title()?,
            "episode": self.get_episode()?,
            "url": self.get_url()?,
            "images": wrap_image(self.get_image_url()?.as_deref()),
        }))
    }
}

fn suffix_digits_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"([\d]+)$").expect("valid regex"))
}

/// Port of `Jikan\Parser\Anime\PromoListItemParser`.
pub struct PromoListItemParser {
    node: HtmlNode,
}

impl PromoListItemParser {
    pub fn new(node: HtmlNode) -> Self {
        PromoListItemParser { node }
    }

    /// `PromoListItemParser::getTitle()`.
    pub fn get_title(&self) -> PResult<String> {
        required_text_node(&self.node, "//a/div/span")
    }

    /// `PromoListItemParser::getImageUrl()`.
    pub fn get_image_url(&self) -> PResult<String> {
        required_node_attr(&self.node, "//a/img", "data-src")
    }

    /// `PromoListItemParser::getVideoUrl()`.
    pub fn get_video_url(&self) -> PResult<String> {
        required_node_attr(&self.node, "//a", "href")
    }

    /// `PromoListItem::fromParser($parser)` serialized by JMS.
    pub fn get_model(&self) -> PResult<Value> {
        Ok(json!({
            "title": self.get_title()?,
            "trailer": youtube_meta(Some(&self.get_video_url()?)),
        }))
    }
}

/// Port of `Jikan\Parser\Anime\MusicVideoListItemParser`.
pub struct MusicVideoListItemParser {
    node: HtmlNode,
}

impl MusicVideoListItemParser {
    pub fn new(node: HtmlNode) -> Self {
        MusicVideoListItemParser { node }
    }

    /// `MusicVideoListItemParser::getTitle()`.
    pub fn get_title(&self) -> PResult<String> {
        required_text_node(&self.node, "//a/div/span")
    }

    /// `MusicVideoListItemParser::getVideoUrl()`.
    pub fn get_video_url(&self) -> PResult<String> {
        required_node_attr(&self.node, "//a", "href")
    }

    /// `MusicVideoListItemParser::getMusic()`.
    pub fn get_music(&self) -> PResult<Value> {
        let Some(node) = self.node.first("//div/div")? else {
            return Ok(music_meta(None, None));
        };
        match music_meta_re().captures(&node.node_text()) {
            Some(caps) => Ok(music_meta(
                caps.get(1).map(|m| m.as_str()),
                caps.get(2).map(|m| m.as_str()),
            )),
            None => Ok(music_meta(None, None)),
        }
    }

    /// `MusicVideoListItem::fromParser($parser)` serialized by JMS.
    pub fn get_model(&self) -> PResult<Value> {
        Ok(json!({
            "title": self.get_title()?,
            "video": youtube_meta(Some(&self.get_video_url()?)),
            "meta": self.get_music()?,
        }))
    }
}

fn music_meta_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(.*) by (.*)").expect("valid regex"))
}

// ---------------------------------------------------------------------------
// CharactersAndStaffParser / StaffListItemParser
// ---------------------------------------------------------------------------

/// Port of `Jikan\Parser\Anime\CharactersAndStaffParser`.
///
/// `getCharacters()` uses the character-list-item parser (a shared
/// `Jikan\Parser\Character` class kept local here to avoid depending on the
/// in-progress character module).
pub struct CharactersAndStaffParser {
    doc: HtmlDoc,
}

impl CharactersAndStaffParser {
    pub fn new(doc: HtmlDoc) -> Self {
        CharactersAndStaffParser { doc }
    }

    /// `CharactersAndStaffParser::getCharacters()`.
    pub fn get_characters(&self) -> PResult<Vec<Value>> {
        let tables = self
            .doc
            .nodes("//div[contains(@class, \"anime-character-container\")]/table")?;
        let mut out = Vec::new();
        for table in &tables {
            out.push(CharacterListItemParser::new(table.clone()).get_model()?);
        }
        Ok(out)
    }

    /// `CharactersAndStaffParser::getStaff()`.
    pub fn get_staff(&self) -> PResult<Vec<Value>> {
        let Some(heading) = self.doc.first("//h2[text()=\"Staff\"]")? else {
            return Ok(Vec::new());
        };
        let Some(parent) = first_ancestor(&heading) else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for sibling in parent.next_all() {
            let has_people =
                sibling.count("//a[contains(@href, \"https://myanimelist.net/people\")]")?;
            if has_people > 0 {
                out.push(StaffListItemParser::new(sibling).get_model()?);
            }
        }
        Ok(out)
    }

    /// `AnimeCharactersAndStaff::fromParser($parser)` serialized by JMS.
    pub fn get_model(&self) -> PResult<Value> {
        Ok(json!({
            "characters": self.get_characters()?,
            "staff": self.get_staff()?,
        }))
    }
}

/// Port of `Jikan\Parser\Anime\StaffListItemParser`.
pub struct StaffListItemParser {
    node: HtmlNode,
}

impl StaffListItemParser {
    pub fn new(node: HtmlNode) -> Self {
        StaffListItemParser { node }
    }

    /// `StaffListItemParser::getPositions()`.
    pub fn get_positions(&self) -> PResult<Vec<String>> {
        let text = required_text_node(&self.node, "//small")?;
        Ok(text.split(", ").map(str::to_string).collect())
    }

    /// `StaffListItemParser::getMalUrl()`: the first anchor without an `<img>`.
    pub fn get_mal_url(&self) -> PResult<Value> {
        let anchor = self.node.nodes("//a")?.into_iter().find(|anchor| {
            anchor
                .count("//img")
                .map(|count| count == 0)
                .unwrap_or(true)
        });
        match anchor {
            Some(anchor) => mal_url_value(&anchor),
            None => Err(ParseError::Html("staff item has no link".to_string())),
        }
    }

    /// `StaffListItemParser::getName()`.
    pub fn get_name(&self) -> PResult<String> {
        Ok(required_attr_from_value(&self.get_mal_url()?, "name"))
    }

    /// `StaffListItemParser::getUrl()`.
    pub fn get_url(&self) -> PResult<String> {
        Ok(required_attr_from_value(&self.get_mal_url()?, "url"))
    }

    /// `StaffListItemParser::getImage()`.
    pub fn get_image(&self) -> PResult<String> {
        let image = required_node_attr(&self.node, "//img", "data-src")?;
        Ok(parse_image_quality(&image))
    }

    /// `StaffListItemParser::getPersonMeta()`.
    pub fn get_person_meta(&self) -> PResult<Value> {
        Ok(person_meta(
            &self.get_name()?,
            &self.get_url()?,
            &self.get_image()?,
        ))
    }

    /// `StaffListItem::fromParser($parser)` serialized by JMS.
    pub fn get_model(&self) -> PResult<Value> {
        Ok(json!({
            "person": self.get_person_meta()?,
            "positions": self.get_positions()?,
        }))
    }
}

fn required_attr_from_value(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// Port of `Jikan\Parser\Character\CharacterListItemParser`.
pub struct CharacterListItemParser {
    node: HtmlNode,
}

impl CharacterListItemParser {
    pub fn new(node: HtmlNode) -> Self {
        CharacterListItemParser { node }
    }

    /// `CharacterListItemParser::getVoiceActors()`.
    pub fn get_voice_actors(&self) -> PResult<Vec<Value>> {
        let rows = self.node.nodes("//table[2]/tr")?;
        let mut out = Vec::new();
        for row in &rows {
            out.push(VoiceActorParser::new(row.clone()).get_model()?);
        }
        Ok(out)
    }

    /// `CharacterListItemParser::getCharacterUrl()`.
    pub fn get_character_url(&self) -> PResult<String> {
        required_node_attr(&self.node, "//td[2]/div[3]/a", "href")
    }

    /// `CharacterListItemParser::getMalId()`.
    pub fn get_mal_id(&self) -> PResult<i64> {
        Ok(id_from_url(&self.get_character_url()?))
    }

    /// `CharacterListItemParser::getName()`.
    pub fn get_name(&self) -> PResult<String> {
        required_text_node(&self.node, "//h3[contains(@class, \"h3_character_name\")]")
    }

    /// `CharacterListItemParser::getImage()`.
    pub fn get_image(&self) -> PResult<String> {
        let image = required_node_attr(&self.node, "//img[1]", "data-src")?;
        Ok(parse_image_quality(&image))
    }

    /// `CharacterListItemParser::getRole()`.
    pub fn get_role(&self) -> PResult<String> {
        let text = required_text_node(&self.node, "//td[2]/div[4]")?;
        Ok(utf8_nbsp_trim(&cleanse(&text)))
    }

    /// `CharacterListItemParser::getFavorites()`.
    pub fn get_favorites(&self) -> PResult<i64> {
        let Some(node) = self.node.first("//td[2]/div[5]")? else {
            return Ok(0);
        };
        Ok(php_int(&node.node_text().replace(',', "")))
    }

    /// `CharacterListItemParser::getCharacterMeta()`.
    pub fn get_character_meta(&self) -> PResult<Value> {
        Ok(character_meta(
            &self.get_name()?,
            &self.get_character_url()?,
            &self.get_image()?,
        ))
    }

    /// `CharacterListItem::fromParser($parser)` serialized by JMS.
    pub fn get_model(&self) -> PResult<Value> {
        let role = self.get_role()?;
        let character = self.get_character_meta()?;
        let favorites = self.get_favorites()?;
        let voice_actors = self.get_voice_actors()?;
        // Mirrors the property declaration order (voiceActors first).
        let mut model = Map::new();
        model.insert("character".to_string(), character);
        model.insert("role".to_string(), json!(role));
        model.insert("favorites".to_string(), json!(favorites));
        model.insert("voice_actors".to_string(), Value::Array(voice_actors));
        Ok(Value::Object(model))
    }
}

/// Port of `Jikan\Parser\Character\VoiceActorParser`.
pub struct VoiceActorParser {
    node: HtmlNode,
}

impl VoiceActorParser {
    pub fn new(node: HtmlNode) -> Self {
        VoiceActorParser { node }
    }

    fn person_anchor(&self) -> PResult<Option<HtmlNode>> {
        Ok(self
            .node
            .nodes("//a")?
            .into_iter()
            .find(|anchor| anchor.count("img").map(|count| count == 0).unwrap_or(true)))
    }

    /// `VoiceActorParser::getName()`.
    pub fn get_name(&self) -> PResult<String> {
        Ok(self
            .person_anchor()?
            .map(|anchor| anchor.node_text())
            .unwrap_or_default())
    }

    /// `VoiceActorParser::getUrl()`.
    pub fn get_url(&self) -> PResult<String> {
        required_node_attr(&self.node, "//a", "href")
    }

    /// `VoiceActorParser::getMalId()`.
    pub fn get_mal_id(&self) -> PResult<i64> {
        Ok(id_from_url(&self.get_url()?))
    }

    /// `VoiceActorParser::getImage()`.
    pub fn get_image(&self) -> PResult<String> {
        let image = self
            .node
            .attr("//img", "src")?
            .or(self.node.attr("//img", "data-src")?)
            .unwrap_or_default();
        Ok(parse_image_quality(&image))
    }

    /// `VoiceActorParser::getLanguage()`.
    pub fn get_language(&self) -> PResult<String> {
        if let Some(node) = self
            .node
            .first("//div[contains(@class, \"js-anime-character-language\")]")?
        {
            return Ok(utf8_nbsp_trim(&cleanse(&node.node_text())));
        }
        let text = required_text_node(&self.node, "//div/small")?;
        Ok(utf8_nbsp_trim(&cleanse(&text)))
    }

    /// `VoiceActorParser::getPersonMeta()`.
    pub fn get_person_meta(&self) -> PResult<Value> {
        Ok(person_meta(
            &self.get_name()?,
            &self.get_url()?,
            &self.get_image()?,
        ))
    }

    /// `VoiceActor::fromParser($parser)` serialized by JMS.
    pub fn get_model(&self) -> PResult<Value> {
        Ok(json!({
            "person": self.get_person_meta()?,
            "language": self.get_language()?,
        }))
    }
}

// ---------------------------------------------------------------------------
// MoreInfoParser
// ---------------------------------------------------------------------------

/// Port of `Jikan\Parser\Anime\MoreInfoParser`.
pub struct MoreInfoParser {
    doc: HtmlDoc,
}

impl MoreInfoParser {
    pub fn new(doc: HtmlDoc) -> Self {
        MoreInfoParser { doc }
    }

    /// `MoreInfoParser::getMoreInfo()`.
    pub fn get_more_info(&self) -> PResult<Option<String>> {
        let Some(node) = self.doc.first("//div[contains(@class, \"rightside\")]")? else {
            return Ok(None);
        };
        node.remove_child_nodes()?;
        let more_info = cleanse(&node.node_text());
        if more_info.is_empty() {
            Ok(None)
        } else {
            Ok(Some(more_info))
        }
    }

    /// `AnimeMoreInfo::fromParser($parser)` serialized by JMS.
    pub fn get_model(&self) -> PResult<Value> {
        Ok(json!({ "more_info": self.get_more_info()? }))
    }
}

// ---------------------------------------------------------------------------
// AnimeStatsParser
// ---------------------------------------------------------------------------

/// Port of `Jikan\Parser\Anime\AnimeStatsParser`.
pub struct AnimeStatsParser {
    doc: HtmlDoc,
}

impl AnimeStatsParser {
    pub fn new(doc: HtmlDoc) -> Self {
        AnimeStatsParser { doc }
    }

    fn sanitized_count(&self, label: &str) -> i64 {
        let xpath = format!(
            "//div[@class=\"spaceit_pad\"]/span[contains(text(), {})]",
            xpath_literal(label)
        );
        match self
            .doc
            .first(&xpath)
            .ok()
            .flatten()
            .and_then(|span| first_ancestor(&span))
        {
            Some(ancestor) => digit_only_int(&ancestor.node_text_raw()),
            None => 0,
        }
    }

    /// `AnimeStatsParser::getWatching()`.
    pub fn get_watching(&self) -> i64 {
        self.sanitized_count("Watching:")
    }

    /// `AnimeStatsParser::getCompleted()`.
    pub fn get_completed(&self) -> i64 {
        self.sanitized_count("Completed:")
    }

    /// `AnimeStatsParser::getOnHold()`.
    pub fn get_on_hold(&self) -> i64 {
        self.sanitized_count("On-Hold:")
    }

    /// `AnimeStatsParser::getDropped()`.
    pub fn get_dropped(&self) -> i64 {
        self.sanitized_count("Dropped:")
    }

    /// `AnimeStatsParser::getPlanToWatch()`.
    pub fn get_plan_to_watch(&self) -> i64 {
        self.sanitized_count("Plan to Watch:")
    }

    /// `AnimeStatsParser::getTotal()`.
    pub fn get_total(&self) -> i64 {
        self.sanitized_count("Total:")
    }

    /// `AnimeStatsParser::getScores()`.
    pub fn get_scores(&self) -> PResult<Value> {
        let marker = self
            .doc
            .nodes("//h2[text()=\"Score Stats\"]/following-sibling::text()")?;
        if let Some(node) = marker.first() {
            if node
                .node_text()
                .contains("No scores have been recorded for this")
            {
                return Ok(json!({}));
            }
        }

        let mut scores: Vec<(i64, Value)> = Vec::new();
        let rows = self
            .doc
            .nodes("//h2[text()=\"Score Stats\"]/following-sibling::table[1]/tr")?;
        for row in &rows {
            let score = php_int(&required_text_node(row, "//td[1]")?);
            let votes = row
                .first("//td[2]/div/span/small")?
                .map(|node| digit_only_int(&node.node_text()))
                .unwrap_or(0);
            let percentage = match row.first("//td[2]/div/span")? {
                Some(node) => {
                    node.remove_child_nodes()?;
                    let value = node.node_text().replace('%', "");
                    php_float(&cleanse(&utf8_nbsp_trim(&value)))
                }
                None => 0.0,
            };
            scores.retain(|(existing, _)| *existing != score);
            scores.push((
                score,
                json!({
                    "score": score,
                    "votes": votes,
                    "percentage": percentage,
                }),
            ));
        }

        let mut model = Map::new();
        for score in 1..=10i64 {
            let value = scores
                .iter()
                .find(|(existing, _)| *existing == score)
                .map(|(_, value)| value.clone())
                .unwrap_or_else(|| json!({ "score": score, "votes": 0, "percentage": 0.0 }));
            model.insert(score.to_string(), value);
        }
        Ok(Value::Object(model))
    }

    /// `AnimeStats::fromParser($parser)` serialized by JMS.
    pub fn get_model(&self) -> PResult<Value> {
        Ok(json!({
            "watching": self.get_watching(),
            "completed": self.get_completed(),
            "on_hold": self.get_on_hold(),
            "dropped": self.get_dropped(),
            "plan_to_watch": self.get_plan_to_watch(),
            "total": self.get_total(),
            "scores": self.get_scores()?,
        }))
    }
}

// ---------------------------------------------------------------------------
// AnimeReviewsParser
// ---------------------------------------------------------------------------

/// Port of `Jikan\Parser\Anime\AnimeReviewsParser`.
pub struct AnimeReviewsParser {
    doc: HtmlDoc,
}

impl AnimeReviewsParser {
    pub fn new(doc: HtmlDoc) -> Self {
        AnimeReviewsParser { doc }
    }

    /// `AnimeReviewsParser::getResults()`.
    pub fn get_results(&self) -> PResult<Vec<Value>> {
        let nodes = self.doc.nodes(
            "//div[contains(@class, \"rightside\")]//div[contains(@class, \"review-element\")]",
        )?;
        let mut out = Vec::new();
        for node in &nodes {
            out.push(AnimeReviewItemParser::new(node.clone()).get_model()?);
        }
        Ok(out)
    }

    /// `AnimeReviewsParser::hasNextPage()`.
    pub fn has_next_page(&self) -> PResult<bool> {
        let count = self
            .doc
            .count("//*[@id=\"content\"]/table//a[contains(text(), \"More Reviews\")]")?;
        Ok(count > 0)
    }

    /// `AnimeReviews::fromParser($parser)` serialized by JMS.
    pub fn get_model(&self) -> PResult<Value> {
        Ok(json!({
            "results": self.get_results()?,
            "has_next_page": self.has_next_page()?,
            "last_visible_page": 1,
        }))
    }
}

/// Port of `Jikan\Parser\Reviews\AnimeReviewParser` (the anime-specific model).
pub struct AnimeReviewItemParser {
    node: HtmlNode,
}

impl AnimeReviewItemParser {
    pub fn new(node: HtmlNode) -> Self {
        AnimeReviewItemParser { node }
    }

    fn field_first(&self, xpath: &str) -> PResult<Option<HtmlNode>> {
        self.node.first(xpath)
    }

    /// `AnimeReviewParser::getId()`.
    pub fn get_id(&self) -> PResult<i64> {
        let url = self.get_url()?;
        Ok(query_param(&url, "id")
            .map(|value| php_int(&value))
            .unwrap_or(0))
    }

    /// `AnimeReviewParser::getUrl()`.
    pub fn get_url(&self) -> PResult<String> {
        required_node_attr(
            &self.node,
            "//div/div[2]/div[contains(@class, \"bottom-navi\")]/div[@class=\"open\"]/a",
            "href",
        )
    }

    /// `AnimeReviewParser::getDate()`.
    pub fn get_date(&self) -> PResult<Option<DateTime<FixedOffset>>> {
        let Some(node) = self.field_first("//div/div[2]/div[contains(@class, \"update_at\")]")?
        else {
            return Ok(None);
        };
        let date = node.node_text();
        let time = node.node_attr("title").unwrap_or_default();
        Ok(date::parse_date(&format!("{date} {time}")))
    }

    /// `AnimeReviewParser::getContent()`.
    pub fn get_content(&self) -> PResult<String> {
        let Some(node) = self.field_first("//div/div[2]/div[contains(@class, \"text\")]")? else {
            return Ok(String::new());
        };
        let expanded = self.field_first(
            "//div/div[2]/div[contains(@class, \"text\")]/span[contains(@class, \"js-hidden\")]",
        )?;
        node.remove_child_nodes()?;
        let mut content = cleanse(&node.node_text());
        if let Some(expanded) = expanded {
            expanded.remove_child_nodes()?;
            content.push_str(&cleanse(&expanded.node_html()));
        }
        Ok(content)
    }

    /// `AnimeReviewParser::getReviewer()`.
    pub fn get_reviewer(&self) -> PResult<Value> {
        let anchor = self.field_first("//div/div[2]/div[contains(@class, \"username\")]/a")?;
        let anchor = match anchor {
            Some(anchor) => Some(anchor),
            None => self.field_first("//div[1]/div[1]/div[4]/table/tr/td[2]/a")?,
        };
        let url = match &anchor {
            Some(anchor) => anchor.node_attr("href").unwrap_or_default(),
            None => String::new(),
        };
        let username = anchor.map(|anchor| anchor.node_text()).unwrap_or_default();

        let image = match self.field_first("//div/div/a/img")? {
            Some(node) => node
                .node_attr("data-src")
                .map(|value| parse_image_thumb_to_hq(&value)),
            None => self
                .field_first("//div[1]/div[1]/div[4]/table/tr/td[1]/div/a/img")?
                .and_then(|node| node.node_attr("src"))
                .map(|value| parse_image_thumb_to_hq(&value)),
        };
        Ok(user_meta(&username, &url, image.as_deref().unwrap_or("")))
    }

    /// `AnimeReviewParser::getType()`.
    pub fn get_type(&self) -> PResult<Option<String>> {
        let node = match self.field_first("//div/div/div[2]/div[2]/small")? {
            Some(node) => Some(node),
            None => self.field_first("//div/small")?,
        };
        Ok(node.map(|node| node.node_text().to_lowercase().replace(['(', ')'], "")))
    }

    /// `AnimeReviewParser::getEpisodesWatched()`.
    pub fn get_episodes_watched(&self) -> PResult<Option<i64>> {
        let Some(node) = self.field_first(
            "//div/div[2]/div[contains(@class, \"tags\")]/div[contains(@class, \"preliminary\")]/span",
        )? else {
            return Ok(None);
        };
        match episodes_re().captures(&cleanse(&node.node_text())) {
            Some(caps) => Ok(Some(php_int(&caps[1]))),
            None => Ok(Some(0)),
        }
    }

    /// `AnimeReviewParser::getReactions()`.
    pub fn get_reactions(&self) -> Value {
        let raw = self
            .node
            .node_attr("data-reactions")
            .map(|value| cleanse(&value))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| {
                "{\"icon\":[],\"num\":0,\"count\":[\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\"]}"
                    .to_string()
            });
        let parsed: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
        let reaction = |index: usize| -> i64 {
            parsed
                .get("count")
                .and_then(|count| count.get(index))
                .and_then(Value::as_str)
                .map(php_int)
                .unwrap_or(0)
        };
        json!({
            "overall": parsed.get("num").map(|n| match n {
                Value::String(s) => php_int(s),
                other => other.as_i64().unwrap_or(0),
            }).unwrap_or(0),
            "nice": reaction(0),
            "love_it": reaction(1),
            "funny": reaction(2),
            "confusing": reaction(3),
            "informative": reaction(4),
            "well_written": reaction(5),
            "creative": reaction(6),
        })
    }

    /// `AnimeReviewParser::getReviewerScore()`.
    pub fn get_reviewer_score(&self) -> PResult<i64> {
        let node = self.field_first("//div/div[2]/div[contains(@class, \"rating\")]/span")?;
        Ok(node.map(|node| php_int(&node.node_text())).unwrap_or(0))
    }

    /// `AnimeReviewParser::getReviewTag()`.
    pub fn get_review_tags(&self) -> PResult<Vec<String>> {
        let nodes = self
            .node
            .nodes("//div/div[2]/div[contains(@class, \"tags\")]/div")?;
        let mut out = Vec::new();
        for node in &nodes {
            node.remove_child_nodes()?;
            out.push(cleanse(&node.node_text()));
        }
        Ok(out)
    }

    /// `AnimeReviewParser::isPreliminary()`.
    pub fn is_preliminary(&self) -> PResult<bool> {
        let count = self.node.count(
            "//div/div[2]/div[contains(@class, \"tags\")]/div[contains(@class, \"preliminary\")]",
        )?;
        Ok(count > 0)
    }

    /// `AnimeReviewParser::isSpoiler()`.
    pub fn is_spoiler(&self) -> PResult<bool> {
        let count = self.node.count(
            "//div/div[2]/div[contains(@class, \"tags\")]/div[contains(@class, \"spoiler\")]",
        )?;
        Ok(count > 0)
    }

    /// `\Jikan\Model\Anime\AnimeReview::fromParser($parser)` serialized by JMS.
    pub fn get_model(&self) -> PResult<Value> {
        Ok(json!({
            "mal_id": self.get_id()?,
            "url": self.get_url()?,
            "type": self.get_type()?.unwrap_or_else(|| "anime".to_string()),
            "reactions": self.get_reactions(),
            "date": self.get_date()?.as_ref().map(date::format_atom),
            "review": self.get_content()?,
            "score": self.get_reviewer_score()?,
            "tags": self.get_review_tags()?,
            "is_spoiler": self.is_spoiler()?,
            "is_preliminary": self.is_preliminary()?,
            "episodes_watched": self.get_episodes_watched()?,
            "user": self.get_reviewer()?,
        }))
    }
}

fn episodes_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\((\d+)/(.*)\)").expect("valid regex"))
}

// ---------------------------------------------------------------------------
// AnimeRecentlyUpdatedByUsersParser
// ---------------------------------------------------------------------------

/// Port of `Jikan\Parser\Anime\AnimeRecentlyUpdatedByUsersParser`.
pub struct AnimeRecentlyUpdatedByUsersParser {
    doc: HtmlDoc,
}

impl AnimeRecentlyUpdatedByUsersParser {
    pub fn new(doc: HtmlDoc) -> Self {
        AnimeRecentlyUpdatedByUsersParser { doc }
    }

    /// `AnimeRecentlyUpdatedByUsersParser::getResults()`.
    pub fn get_results(&self) -> PResult<Vec<Value>> {
        let Some(header) = self
            .doc
            .first("//table[@class=\"table-recently-updated\"]/tr[1]")?
        else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for row in header.next_all() {
            out.push(AnimeRecentlyUpdatedByUsersListParser::new(row).get_model()?);
        }
        Ok(out)
    }

    /// `AnimeRecentlyUpdatedByUsersParser::getHasNextPage()`.
    pub fn get_has_next_page(&self) -> bool {
        false
    }

    /// `AnimeRecentlyUpdatedByUsersParser::getLastPage()`.
    pub fn get_last_page(&self) -> i64 {
        1
    }

    /// `AnimeUserUpdates::fromParser($parser)` serialized by JMS.
    pub fn get_model(&self) -> PResult<Value> {
        Ok(json!({
            "results": self.get_results()?,
            "has_next_page": self.get_has_next_page(),
            "last_visible_page": self.get_last_page(),
        }))
    }
}

/// Port of `Jikan\Parser\Anime\AnimeRecentlyUpdatedByUsersListParser`.
pub struct AnimeRecentlyUpdatedByUsersListParser {
    node: HtmlNode,
}

impl AnimeRecentlyUpdatedByUsersListParser {
    pub fn new(node: HtmlNode) -> Self {
        AnimeRecentlyUpdatedByUsersListParser { node }
    }

    /// `AnimeRecentlyUpdatedByUsersListParser::getUsername()`.
    pub fn get_username(&self) -> PResult<String> {
        required_text_node(&self.node, "//td[1]/div[2]/a")
    }

    /// `AnimeRecentlyUpdatedByUsersListParser::getUrl()`.
    pub fn get_url(&self) -> PResult<String> {
        required_node_attr(&self.node, "//td[1]/div[2]/a", "href")
    }

    /// `AnimeRecentlyUpdatedByUsersListParser::getImageUrl()`.
    pub fn get_image_url(&self) -> PResult<String> {
        let style = required_node_attr(&self.node, "//td[1]/div[1]/a", "style")?;
        Ok(style
            .replace("thumbs/", "")
            .replace("_thumb", "")
            .replace("background-image:url(", "")
            .replace(')', ""))
    }

    /// `AnimeRecentlyUpdatedByUsersListParser::getScore()`.
    pub fn get_score(&self) -> PResult<Option<i64>> {
        let text = required_text_node(&self.node, "//td[2]")?;
        if text == "-" {
            return Ok(None);
        }
        Ok(Some(php_int(&text)))
    }

    /// `AnimeRecentlyUpdatedByUsersListParser::getStatus()`.
    pub fn get_status(&self) -> PResult<String> {
        required_text_node(&self.node, "//td[3]")
    }

    /// `AnimeRecentlyUpdatedByUsersListParser::getEpisodesSeen()`.
    pub fn get_episodes_seen(&self) -> PResult<Option<i64>> {
        let Some(text) = self.episodes_text()? else {
            return Ok(None);
        };
        let seen = text.split('/').next().unwrap_or("").to_string();
        if seen == "-" {
            return Ok(None);
        }
        Ok(Some(php_int(&seen)))
    }

    /// `AnimeRecentlyUpdatedByUsersListParser::getEpisodesTotal()`.
    pub fn get_episodes_total(&self) -> PResult<Option<i64>> {
        let Some(text) = self.episodes_text()? else {
            return Ok(None);
        };
        let total = text.split('/').nth(1).unwrap_or("").to_string();
        if total == "-" {
            return Ok(None);
        }
        Ok(Some(php_int(&total)))
    }

    fn episodes_text(&self) -> PResult<Option<String>> {
        let Some(node) = self.node.first("//td[4]")? else {
            return Ok(None);
        };
        let text = node.node_text().trim().replace(' ', "");
        if text.is_empty() {
            return Ok(None);
        }
        Ok(Some(text))
    }

    /// `AnimeRecentlyUpdatedByUsersListParser::getDate()`.
    pub fn get_date(&self) -> PResult<Option<DateTime<FixedOffset>>> {
        let text = required_text_node(&self.node, "//td[5]")?;
        Ok(date::parse_date(&text))
    }

    /// `AnimeRecentlyUpdatedByUsersListParser::getUserMeta()`.
    pub fn get_user_meta(&self) -> PResult<Value> {
        Ok(user_meta(
            &self.get_username()?,
            &self.get_url()?,
            &self.get_image_url()?,
        ))
    }

    /// `AnimeRecentlyUpdatedByUser::fromParser($parser)` serialized by JMS.
    pub fn get_model(&self) -> PResult<Value> {
        Ok(json!({
            "user": self.get_user_meta()?,
            "score": self.get_score()?,
            "status": self.get_status()?,
            "episodes_seen": self.get_episodes_seen()?,
            "episodes_total": self.get_episodes_total()?,
            "date": self.get_date()?.as_ref().map(date::format_atom),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_image_urls() {
        let image = common_image(Some("https://cdn.myanimelist.net/images/anime/7/20310.jpg"));
        assert_eq!(
            image["jpg"]["small_image_url"],
            "https://cdn.myanimelist.net/images/anime/7/20310t.jpg"
        );
        assert_eq!(
            image["webp"]["large_image_url"],
            "https://cdn.myanimelist.net/images/anime/7/20310l.webp"
        );
    }

    #[test]
    fn youtube_meta_factory() {
        let trailer = youtube_meta(Some(
            "https://www.youtube.com/embed/bJVyIXeUznY?enablejsapi=1&wmode=opaque&autoplay=1",
        ));
        assert_eq!(trailer["youtube_id"], "bJVyIXeUznY");
        assert_eq!(
            trailer["url"],
            "https://www.youtube.com/watch?v=bJVyIXeUznY"
        );
        assert_eq!(
            trailer["images"]["image_url"],
            "https://img.youtube.com/vi/bJVyIXeUznY/default.jpg"
        );
    }

    #[test]
    fn date_range_shape() {
        let range = date_range_value("Apr 1, 1998 to Sep 30, 1998");
        assert_eq!(range["from"], "1998-04-01T00:00:00+00:00");
        assert_eq!(range["to"], "1998-09-30T00:00:00+00:00");
        assert_eq!(range["prop"]["from"]["day"], 1);
        assert_eq!(range["prop"]["to"]["year"], 1998);
        assert_eq!(range["string"], "Apr 1, 1998 to Sep 30, 1998");
    }

    #[test]
    fn date_range_not_available() {
        let range = date_range_value("Not available");
        assert!(range["from"].is_null());
        assert!(range["to"].is_null());
        assert!(range["prop"]["from"]["day"].is_null());
    }

    #[test]
    fn php_casts() {
        assert_eq!(php_int(" 26"), 26);
        assert_eq!(php_int("abc"), 0);
        assert_eq!(php_int("1,234"), 1);
        assert_eq!(php_float("8.22"), 8.22);
        assert_eq!(php_float("abc"), 0.0);
        assert_eq!(digit_only_int("4,286 users"), 4286);
    }

    #[test]
    fn jst_aired_parsing() {
        let dt = parse_date_with_timezone("Oct 20, 1999(JST)").expect("JST date");
        assert_eq!(date::format_atom(&dt), "1999-10-20T00:00:00+09:00");
    }
}
