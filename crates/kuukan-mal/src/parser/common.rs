//! Shared `Jikan\Parser\Common` parsers.
//!
//! Ports of the classes in `/tmp/opencode/jikan-php/src/Parser/Common/` that
//! the other parser families call:
//!
//! | PHP | Rust |
//! |---|---|
//! | `MalUrlParser` | [`mal_url`], [`mal_url_from_parts`], [`parse_mal_id`] |
//! | `UrlParser` | [`url_parser`] |
//! | `AlternativeTitleParser` | [`alternative_titles`] |
//! | `PictureParser` | [`picture`] |
//! | `PicturesPageParser` | [`pictures_page`] |
//! | `DefaultPicturesPageParser` | [`default_pictures_page`] |
//! | `ItemMetaParser` | [`item_meta`] |
//! | `AnimeCardParser` | [`anime_card`] (+ [`anime_card_continuing`]) |
//! | `MangaCardParser` | [`manga_card`] |
//! | `Recommendation` / `Recommendations` | [`recommendation`], [`recommendations`] |
//!
//! Every function returns the JMS-shaped JSON of the matching model: keys in
//! snake_case, `null` preserved, empty arrays kept as `[]`, `MalUrl` as
//! `{mal_id, type, name, url}` (`App\Providers\SerializerFactory::convertMalUrl`).
//!
//! PHP quirks preserved:
//!
//! - `CommonImageResource::factory(null)` leaves the `jpg` URLs `null` but
//!   produces *empty strings* for `webp`: on PHP 8 `str_replace('.jpg', ...,
//!   null)` returns `""`, which is not `null`, so the Webp factory does not
//!   take its early return.
//! - `AnimeCard::type` and `AnimeCard::licensors` are hard-coded `null`/`[]`;
//!   `MangaCard::type` is hard-coded `null` and `MangaCardParser::getDescription()`
//!   is typed `string` (a missing `<p>` raises a PHP `TypeError`); Kuukan yields
//!   `""` there.
//! - `AnimeCardParser::getMembers()` has no final `return` when neither the
//!   information nor the widget node exists (PHP `TypeError`); Kuukan yields
//!   `0` for a graceful degradation. `MangaCardParser::getMembers()` and
//!   `getMangaScore()` call `text()` on nodes that must exist (PHP
//!   `InvalidArgumentException`); Kuukan yields `0`/`null` when they are absent.
//! - `MangaCardParser::getPublishDates()` feeds the trailing `, <digits>` of
//!   "Manga, 2009" straight into `DateTimeImmutable`, so a 4-digit value that
//!   parses as `HH:MM` (e.g. `2009`) becomes *today's* date instead of a year.
//! - `AnimeCardParser::getThemes()`/`getDemographics()`/`getProducer()` call
//!   `nextAll()` on the label `<span>` itself (not its parent div), exactly
//!   like the PHP.
//! - `Picture`/`Recommendation` attribute lookups throw on an empty crawler in
//!   PHP (`InvalidArgumentException`); Kuukan returns [`ParseError::InvalidXPath`]
//!   in that case. A present-but-empty attribute stays an empty string.

use chrono::{DateTime, Datelike, Duration, FixedOffset, NaiveDate, TimeZone, Timelike, Utc};
use serde_json::{json, Value};

use crate::error::ParseError;
use crate::parser::date::format_atom;
use crate::parser::helper::{parse_image_quality, HtmlDoc, HtmlNode};
use crate::parser::jstring::cleanse;
use crate::parser::mal_url::{id_from_url, MalUrl, MalUrlParser, BASE_URL};

// ---------------------------------------------------------------------------
// MalUrlParser / UrlParser
// ---------------------------------------------------------------------------

/// `MalUrlParser::parseId()`: first `/(\d+)` group in the URL, `0` when absent.
pub fn parse_mal_id(url: &str) -> i64 {
    MalUrlParser::parse_id(url)
}

/// `(new MalUrlParser($node))->getModel()` as JSON: `{mal_id, type, name, url}`.
///
/// The href is normalized exactly like the PHP (`str_replace` of the MAL base
/// URL) and the name goes through `JString::cleanse`.
pub fn mal_url(node: &HtmlNode) -> Result<Value, ParseError> {
    Ok(MalUrlParser::new(node.clone()).get_model()?.to_json())
}

/// Build a `MalUrl` JSON object from an href/text pair without a DOM node.
///
/// Mirrors `MalUrlParser::getModel()`: the MAL base URL is stripped and
/// re-prepended (so relative hrefs become absolute) and `text` is cleansed.
/// `kind` overrides the `type` field (which is otherwise derived from the URL
/// by `MalUrl::getType()`), for callers whose href does not match
/// `https://myanimelist.net/<type>/...`.
pub fn mal_url_from_parts(href: &str, text: &str, kind: Option<&str>) -> Value {
    let href = href.replace(BASE_URL, "");
    let url = format!("{BASE_URL}{href}");
    let mut value = MalUrl::new(cleanse(text), url).to_json();
    if let Some(kind) = kind {
        value["type"] = Value::String(kind.to_string());
    }
    value
}

/// `(new UrlParser($node))->getModel()` as JSON: `{name, url}`.
///
/// Unlike `MalUrlParser`, the href is cleansed too; a missing attribute is
/// `null` in PHP and is coerced to `""` here.
pub fn url_parser(node: &HtmlNode) -> Result<Value, ParseError> {
    Ok(json!({
        "name": cleanse(&node.node_text()),
        "url": cleanse(&node.node_attr("href").unwrap_or_default()),
    }))
}

// ---------------------------------------------------------------------------
// AlternativeTitleParser
// ---------------------------------------------------------------------------

/// Port of `AnimeParser::getTitles()` / `MangaParser::getTitles()`'s
/// `AlternativeTitleParser` part: the `Japanese:` / `English:` / `Synonyms:`
/// entries of the "Alternative Titles" section, flattened.
///
/// The caller is responsible for prepending the `Default` title (which the PHP
/// family parsers do before merging these entries).
pub fn alternative_titles(doc: &HtmlDoc) -> Result<Vec<Value>, ParseError> {
    let containers = doc.nodes(
        "//h2[text()=\"Alternative Titles\"]/following-sibling::div[following::h2[text()=\"Information\"]]",
    )?;

    let mut titles = Vec::new();
    for container in containers {
        for item in container.nodes("//div[contains(@class, \"spaceit_pad\")]")? {
            titles.extend(alternative_title_entries(&item.node_text()));
        }
    }
    Ok(titles)
}

/// `AlternativeTitleParser::getModel()` for one `div.spaceit_pad`.
fn alternative_title_entries(text: &str) -> Vec<Value> {
    let mut parts = text.splitn(2, ':');
    let title_type = parts.next().unwrap_or_default();
    // PHP list-destructuring leaves `$title` null when there is no colon;
    // `JString::cleanse(null)` coerces to "" in non-strict mode.
    let title = parts.next().unwrap_or_default();

    if title_type != "Synonyms" {
        return vec![json!({"type": title_type, "title": cleanse(title)})];
    }

    title
        .split(", ")
        .map(|synonym| json!({"type": "Synonym", "title": cleanse(synonym)}))
        .collect()
}

// ---------------------------------------------------------------------------
// PictureParser / PicturesPageParser / DefaultPicturesPageParser
// ---------------------------------------------------------------------------

/// `(new PictureParser($node))->getModel()` as JSON:
/// `{image_url, large_image_url}` (`image_url` is the `data-src` thumbnail,
/// `large_image_url` the `<a href>`).
pub fn picture(node: &HtmlNode) -> Result<Value, ParseError> {
    let (image_url, large_image_url) = picture_urls(node)?;
    Ok(json!({
        "image_url": image_url,
        "large_image_url": large_image_url,
    }))
}

/// `PicturesPageParser::getModel()`: every `a.js-picture-gallery` as a
/// `CommonImageResource` built from its `data-src` thumbnail.
pub fn pictures_page(doc: &HtmlDoc) -> Result<Vec<Value>, ParseError> {
    let mut pictures = Vec::new();
    for anchor in doc.nodes("//a[@class=\"js-picture-gallery\"]")? {
        let (image_url, _) = picture_urls(&anchor)?;
        pictures.push(common_image_resource_str(&image_url));
    }
    Ok(pictures)
}

/// `DefaultPicturesPageParser::getModel()`: every `a.js-picture-gallery` as a
/// `PersonImageResource` (only `jpg.image_url`).
///
/// `DefaultPicture::fromParser()` only asks for `getSmall()` (`data-src`), so
/// the `<a href>` is not required here (unlike [`pictures_page`]).
pub fn default_pictures_page(doc: &HtmlDoc) -> Result<Vec<Value>, ParseError> {
    let mut pictures = Vec::new();
    for anchor in doc.nodes("//a[@class=\"js-picture-gallery\"]")? {
        let image_url = required_attr(&anchor, "//img", "data-src")?;
        pictures.push(json!({"jpg": {"image_url": image_url}}));
    }
    Ok(pictures)
}

/// `PictureParser::getLarge()` + `getSmall()` (the latter is `Picture`'s
/// `image_url`). Both are required in PHP (`attr()` throws on an empty
/// crawler); a missing node/attribute is an [`ParseError::InvalidXPath`].
fn picture_urls(node: &HtmlNode) -> Result<(String, String), ParseError> {
    let large = required_attr(node, "//a", "href")?;
    let small = required_attr(node, "//img", "data-src")?;
    Ok((small, large))
}

// ---------------------------------------------------------------------------
// ItemMetaParser
// ---------------------------------------------------------------------------

/// `ItemMetaParser` getters as the `Jikan\Model\Common\ItemMeta` JSON:
/// `{mal_id, url, image_url, name}`.
///
/// The getters are declared `?string`, so a missing `<a>`/`<img>` stays `null`
/// (the concrete PHP model would raise a `TypeError`, the closest analogue of
/// which is a null in Kuukan's dynamic payload).
pub fn item_meta(node: &HtmlNode) -> Result<Value, ParseError> {
    let url = node
        .first("//a")?
        .and_then(|anchor| anchor.node_attr("href"));
    let name = node.first("//a")?.map(|anchor| anchor.node_text());
    let image = node
        .first("//img")?
        .and_then(|image| image.node_attr("data-src"))
        .map(|src| parse_image_quality(&src));

    let mal_id = match &url {
        Some(url) => item_meta_mal_id(url),
        None => 0,
    };

    Ok(json!({
        "mal_id": mal_id,
        "url": url,
        "image_url": image,
        "name": name,
    }))
}

/// `ItemMetaParser::getMalId()`: `(int) preg_replace('#https://myanimelist.net/\w+/(\d+).*#', '$1', $url)`.
fn item_meta_mal_id(url: &str) -> i64 {
    let re = item_meta_id_re();
    match re.captures(url) {
        Some(caps) => caps[1].parse().unwrap_or(0),
        // No match: preg_replace returns the subject unchanged and PHP casts it.
        None => php_intval(url),
    }
}

// ---------------------------------------------------------------------------
// AnimeCardParser
// ---------------------------------------------------------------------------

/// `(new AnimeCardParser($node))->getModel()` as the JMS-shaped
/// `Jikan\Model\Common\AnimeCard` JSON.
pub fn anime_card(node: &HtmlNode) -> Result<Value, ParseError> {
    let url = anime_url(node)?;
    let image = anime_image(node)?;
    let score = anime_score(node)?;
    let airing_start = anime_air_dates(node)?;

    Ok(json!({
        "mal_id": id_from_url(&url),
        "url": url,
        "title": anime_title(node)?,
        "images": common_image_resource(image.as_deref()),
        "synopsis": node_text_opt(node, "//div[contains(@class, \"synopsis\")]/p")?,
        // AnimeCardParser::getType() returns null before reading anything.
        "type": Value::Null,
        "airing_start": airing_start,
        "episodes": anime_episodes(node)?,
        "members": anime_members(node)?,
        "genres": mal_urls(node, "//span[@class=\"genre\"]/a")?,
        "explicit_genres": mal_urls(node, "//span[@class=\"genre explicit\"]/a")?,
        "themes": grouped_mal_urls(node, 3, "Theme")?,
        "demographics": grouped_mal_urls(node, 4, "Demographic")?,
        "source": node_text_opt(
            node,
            "//div[contains(@class, \"synopsis\")]/div[contains(@class, \"properties\")]/div[2]/span[2]",
        )?,
        "producers": grouped_mal_urls(node, 1, "Studio")?,
        "score": score,
        // AnimeCardParser::getLicensors() returns [] before reading anything.
        "licensors": Vec::<Value>::new(),
        "r18": has_class(node, "r18"),
        "kids": has_class(node, "kids"),
    }))
}

/// `AnimeCardParser::isContinuing()`: `strpos($node->ancestors()->text(), '(Continuing)') !== false`.
///
/// Needed by the seasonal crew for `Jikan\Model\Seasonal\SeasonalAnime`
/// (`continuing`); the `AnimeCard` payload itself has no such field.
pub fn anime_card_continuing(node: &HtmlNode) -> bool {
    match node.ancestors().first() {
        Some(ancestor) => ancestor.node_text().contains("(Continuing)"),
        None => false,
    }
}

/// `AnimeCardParser::getAnimeUrl()`.
fn anime_url(node: &HtmlNode) -> Result<String, ParseError> {
    if let Some(title) = node.first("//div/div/h2/a")? {
        return Ok(title.node_attr("href").unwrap_or_default());
    }
    Ok(node
        .first("//div[contains(@class, \"title\")]/a")?
        .and_then(|title| title.node_attr("href"))
        .unwrap_or_default())
}

/// `AnimeCardParser::getTitle()`.
fn anime_title(node: &HtmlNode) -> Result<String, ParseError> {
    if let Some(title) = node.first("//div/div/h2/a")? {
        return Ok(title.node_text());
    }
    Ok(node
        .first("//div[contains(@class, \"title\")]/a")?
        .map(|title| title.node_text())
        .unwrap_or_default())
}

/// `AnimeCardParser::getAnimeImage()`.
fn anime_image(node: &HtmlNode) -> Result<Option<String>, ParseError> {
    let Some(image) = node.first("//div[contains(@class, \"image\")]/a/img")? else {
        return Ok(None);
    };
    if let Some(src) = image.node_attr("src") {
        return Ok(Some(parse_image_quality(&src)));
    }
    if let Some(src) = image.node_attr("data-src") {
        return Ok(Some(parse_image_quality(&src)));
    }
    Ok(None)
}

/// `AnimeCardParser::getEpisodes()`.
fn anime_episodes(node: &HtmlNode) -> Result<Option<i64>, ParseError> {
    let episodes = match node
        .first("//div/div[2]/div[2]/span[contains(@class, \"item\")][2]/span[1]")?
    {
        Some(node) => Some(node),
        None => node.first("//div/div[2]/div[2]/span[contains(@class, \"item\")][3]/span[1]")?,
    };
    let Some(episodes) = episodes else {
        return Ok(None);
    };

    let text = cleanse(&episodes.node_text()).replace(" eps", "");
    if text == "?" {
        Ok(None)
    } else {
        Ok(Some(php_intval(&text)))
    }
}

/// `AnimeCardParser::getAirDates()` as a `DATE_ATOM` string.
fn anime_air_dates(node: &HtmlNode) -> Result<Option<String>, ParseError> {
    let Some(dates) = node.first("//div/div[2]/div[2]/span[contains(@class, \"item\")][1]")? else {
        return Ok(None);
    };
    let date = cleanse(&dates.node_text()).replace("(JST)", "");
    Ok(parse_jst_datetime(&date).map(|date| format_atom(&date.with_timezone(&Utc))))
}

/// `AnimeCardParser::getMembers()`.
///
/// PHP falls off the end (fatal `TypeError`) when neither node matches; Kuukan
/// returns `0` instead of a `ParseError`.
fn anime_members(node: &HtmlNode) -> Result<i64, ParseError> {
    if let Some(count) = node.first("//div[contains(@class, \"information\")]/div/div/div[2]")? {
        return Ok(parse_short_count(&count.node_text()));
    }
    if let Some(count) = node.first("//div[contains(@class, \"widget\")]/div[@class=\"users\"]")? {
        return Ok(parse_short_count(&count.node_text()));
    }
    Ok(0)
}

/// `AnimeCardParser::getAnimeScore()` (falls back to the widget stars).
fn anime_score(node: &HtmlNode) -> Result<Option<f64>, ParseError> {
    let mut score = None;
    if let Some(node) = node.first("//div[contains(@class, \"information\")]/div/div/div[1]")? {
        let text = cleanse(&node.node_text());
        if text == "N/A" {
            return Ok(None);
        }
        score = Some(text);
    }
    if let Some(node) = node.first("//div[contains(@class, \"widget\")]/div[@class=\"stars\"]")? {
        let text = cleanse(&node.node_text());
        if text == "N/A" {
            return Ok(None);
        }
        score = Some(text);
    }
    // PHP casts an undefined `$score` (null) to 0.0.
    Ok(Some(match score {
        Some(score) => php_floatval(&score),
        None => 0.0,
    }))
}

// ---------------------------------------------------------------------------
// MangaCardParser
// ---------------------------------------------------------------------------

/// `(new MangaCardParser($node))->getModel()` as the JMS-shaped
/// `Jikan\Model\Common\MangaCard` JSON.
pub fn manga_card(node: &HtmlNode) -> Result<Value, ParseError> {
    let url = manga_url(node)?;
    let image = manga_image(node)?;
    let score = manga_score(node)?;
    let publishing_start = manga_publish_dates(node)?.map(|date| format_atom(&date));

    Ok(json!({
        "mal_id": id_from_url(&url),
        "url": url,
        "title": manga_title(node)?,
        "images": common_image_resource(image.as_deref()),
        "synopsis": node_text_opt(node, "//div[contains(@class, \"synopsis\")]/p")?.unwrap_or_default(),
        // MangaCardParser::getType() returns null before reading anything.
        "type": Value::Null,
        "publishing_start": publishing_start,
        "volumes": manga_volumes(node)?,
        "members": manga_members(node)?,
        "genres": mal_urls(node, "//span[@class=\"genre\"]/a")?,
        "explicit_genres": mal_urls(node, "//span[@class=\"genre explicit\"]/a")?,
        "themes": grouped_mal_urls(node, 3, "Theme")?,
        "demographics": grouped_mal_urls(node, 4, "Demographic")?,
        "authors": mal_urls(node, "//span[contains(@class, \"producer\")]/a")?,
        "score": score,
        "serialization": manga_serialization(node)?,
    }))
}

/// `MangaCardParser::getMangaUrl()`.
fn manga_url(node: &HtmlNode) -> Result<String, ParseError> {
    Ok(node
        .first("//div/div/h2/a")?
        .and_then(|title| title.node_attr("href"))
        .unwrap_or_default())
}

/// `MangaCardParser::getTitle()`.
fn manga_title(node: &HtmlNode) -> Result<String, ParseError> {
    Ok(node
        .first("//div/div/h2/a")?
        .map(|title| title.node_text())
        .unwrap_or_default())
}

/// `MangaCardParser::getMangaImage()` (note: no `/a/` in the XPath).
fn manga_image(node: &HtmlNode) -> Result<Option<String>, ParseError> {
    let Some(image) = node.first("//div[contains(@class, \"image\")]/img")? else {
        return Ok(None);
    };
    if let Some(src) = image.node_attr("src") {
        return Ok(Some(parse_image_quality(&src)));
    }
    if let Some(src) = image.node_attr("data-src") {
        return Ok(Some(parse_image_quality(&src)));
    }
    Ok(None)
}

/// `MangaCardParser::getVolumes()`: first `[0-9]+` of the third (then second)
/// `span.item`, or `null`.
fn manga_volumes(node: &HtmlNode) -> Result<Option<i64>, ParseError> {
    let volumes = match node.first("//div/div[2]/div/span[contains(@class, \"item\")][3]")? {
        Some(node) => Some(node),
        None => node.first("//div/div[2]/div/span[contains(@class, \"item\")][2]")?,
    };
    let Some(volumes) = volumes else {
        return Ok(None);
    };

    let text = cleanse(&volumes.node_text());
    match digits_re().captures(&text) {
        Some(caps) => Ok(Some(caps[1].parse().unwrap_or(0))),
        None => Ok(None),
    }
}

/// `MangaCardParser::getPublishDates()`.
fn manga_publish_dates(node: &HtmlNode) -> Result<Option<DateTime<FixedOffset>>, ParseError> {
    let Some(dates) = node.first("//div/div[2]/div/span[contains(@class, \"item\")][1]")? else {
        return Ok(None);
    };

    // preg_match('~(.*), ([0-9]{1,})$~', $node->text(), $matches); $date = $matches[2];
    let text = dates.node_text();
    let Some((_, digits)) = text.rsplit_once(", ") else {
        return Ok(None);
    };
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Ok(None);
    }

    // (new DateTimeImmutable($date, JST))->setTimezone(UTC)->setTime(0, 0)
    Ok(parse_jst_digits(digits).map(|date| {
        let utc = date.with_timezone(&Utc);
        let midnight = utc
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .expect("midnight is always valid");
        Utc.from_utc_datetime(&midnight).fixed_offset()
    }))
}

/// `MangaCardParser::getMembers()`.
///
/// PHP calls `text()` on a node that must exist (an empty crawler throws);
/// Kuukan yields `0` when it does not.
fn manga_members(node: &HtmlNode) -> Result<i64, ParseError> {
    let count = node
        .first("//div[contains(@class, \"information\")]/div/div/div[2]")?
        .map(|count| cleanse(&count.node_text()))
        .unwrap_or_default();
    Ok(parse_short_count(&count))
}

/// `MangaCardParser::getMangaScore()`.
fn manga_score(node: &HtmlNode) -> Result<Option<f64>, ParseError> {
    let Some(score) = node.first("//div[contains(@class, \"information\")]/div/div/div[1]")? else {
        return Ok(None);
    };
    let score = cleanse(&score.node_text());
    if score == "N/A" {
        return Ok(None);
    }
    Ok(Some(php_floatval(&score)))
}

/// `MangaCardParser::getSerialization()`: plain strings, not `MalUrl`s.
fn manga_serialization(node: &HtmlNode) -> Result<Vec<Value>, ParseError> {
    let mut serialization = Vec::new();
    for item in
        node.nodes("//div[contains(@class, \"synopsis\")]//p[contains(@class, \"mb4 mt8\")]")?
    {
        let Some(label) = item.first("//span")? else {
            continue;
        };
        if !label.node_text().contains("Serialization") {
            continue;
        }
        for sibling in label.next_all() {
            for anchor in sibling.nodes("//a")? {
                serialization.push(Value::String(anchor.node_text()));
            }
        }
    }
    Ok(serialization)
}

// ---------------------------------------------------------------------------
// Recommendation / Recommendations
// ---------------------------------------------------------------------------

/// `(new Recommendation($node))->getModel()` as the JMS-shaped JSON:
/// `{entry: {mal_id, url, images, title}, url, votes}`.
pub fn recommendation(node: &HtmlNode) -> Result<Value, ParseError> {
    let url = required_attr(node, "//table/tr/td[2]/div[2]/a[1]", "href")?;
    let image_url = required_attr(node, "//table/tr/td[1]/div[1]/a/img", "data-src")?;
    let recommendation_url = format!(
        "{BASE_URL}{}",
        required_attr(node, "//table/tr/td[2]/div[2]/span/a", "href")?
    );
    let title = node
        .first("//table/tr/td[2]/div[2]/a[1]")?
        .map(|title| title.node_text())
        .unwrap_or_default();

    let votes = match node.first("//table/tr/td[2]/div[4]/a[1]/strong")? {
        Some(count) => php_intval(&count.node_text()) + 1,
        None => 1,
    };

    // CommonMeta::__construct parses the image quality again; parseImageQuality
    // is idempotent, so once is enough.
    let entry = json!({
        "mal_id": id_from_url(&url),
        "url": url,
        "images": common_image_resource_str(&parse_image_quality(&image_url)),
        "title": title,
    });

    Ok(json!({
        "entry": entry,
        "url": recommendation_url,
        "votes": votes,
    }))
}

/// `(new Recommendations($doc))->getModel()`: every `div.borderClass`.
pub fn recommendations(doc: &HtmlDoc) -> Result<Vec<Value>, ParseError> {
    let mut recommendations = Vec::new();
    for node in doc.nodes("//div[@class=\"borderClass\"]")? {
        recommendations.push(recommendation(&node)?);
    }
    Ok(recommendations)
}

// ---------------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------------

/// `CommonImageResource::factory($imageUrl)`.
fn common_image_resource(image_url: Option<&str>) -> Value {
    match image_url {
        // Jpg::factory(null) early-returns, but Webp::factory(null) assigns
        // `str_replace(..., null)` == "" and keeps going: the webp URLs are
        // empty strings, not null.
        None => json!({
            "jpg": {
                "image_url": Value::Null,
                "small_image_url": Value::Null,
                "large_image_url": Value::Null,
            },
            "webp": {
                "image_url": "",
                "small_image_url": "",
                "large_image_url": "",
            },
        }),
        Some(image_url) => json!({
            "jpg": {
                "image_url": image_url,
                "small_image_url": image_url.replace(".jpg", "t.jpg"),
                "large_image_url": image_url.replace(".jpg", "l.jpg"),
            },
            "webp": {
                "image_url": image_url.replace(".jpg", ".webp"),
                "small_image_url": image_url.replace(".jpg", "t.webp"),
                "large_image_url": image_url.replace(".jpg", "l.webp"),
            },
        }),
    }
}

/// `CommonImageResource::factory()` for a non-null URL.
fn common_image_resource_str(image_url: &str) -> Value {
    common_image_resource(Some(image_url))
}

/// Every `<a>` matched by `xpath` (relative to `node`) as a `MalUrl`.
fn mal_urls(node: &HtmlNode, xpath: &str) -> Result<Vec<Value>, ParseError> {
    let mut urls = Vec::new();
    for anchor in node.nodes(xpath)? {
        urls.push(mal_url(&anchor)?);
    }
    Ok(urls)
}

/// The `properties` grouping the PHP reads with `nextAll()`: for every label
/// `<span>` of `div[div_index]` whose text contains `needle`, each following
/// sibling's `<a>` becomes a `MalUrl`.
fn grouped_mal_urls(
    node: &HtmlNode,
    div_index: usize,
    needle: &str,
) -> Result<Vec<Value>, ParseError> {
    let xpath = format!(
        "//div[contains(@class, \"synopsis\")]/div[contains(@class, \"properties\")]/div[{div_index}]/span"
    );

    let mut urls = Vec::new();
    for label in node.nodes(&xpath)? {
        // $node = $c->filterXPath('//span'); (the span itself)
        let Some(label) = label.first("//span")? else {
            continue;
        };
        if !label.node_text().contains(needle) {
            continue;
        }
        for sibling in label.next_all() {
            for anchor in sibling.nodes("//a")? {
                urls.push(mal_url(&anchor)?);
            }
        }
    }
    Ok(urls)
}

/// `$crawler->filterXPath($xpath)->text()` as `Option` (missing = null).
fn node_text_opt(node: &HtmlNode, xpath: &str) -> Result<Option<String>, ParseError> {
    Ok(node.first(xpath)?.map(|node| node.node_text()))
}

/// `DomCrawler::attr()` on a required node/attribute. PHP throws
/// `InvalidArgumentException` ("The current node list is empty.") when the
/// node is missing; a missing attribute is null and later a `TypeError`.
fn required_attr(node: &HtmlNode, xpath: &str, attr: &str) -> Result<String, ParseError> {
    node.first(xpath)?
        .and_then(|node| node.node_attr(attr))
        .ok_or_else(|| ParseError::InvalidXPath(format!("{xpath} has no @{attr}")))
}

/// `isR18()` / `isKids()`: `in_array($class, explode(' ', $crawler->attr('class')), true)`.
fn has_class(node: &HtmlNode, class: &str) -> bool {
    node.node_attr("class")
        .map(|classes| classes.split(' ').any(|value| value == class))
        .unwrap_or(false)
}

/// `AnimeCardParser`'s member text: cleanse, `K` -> `000`, `M` -> `000000`,
/// then strip `,`/`.` and cast to int.
fn parse_short_count(text: &str) -> i64 {
    let count = cleanse(text).replace('K', "000").replace('M', "000000");
    php_intval(&count.replace([',', '.'], ""))
}

/// `JString::cleanse($node->text())` on the air/publish date, then
/// `new \DateTimeImmutable($date, new \DateTimeZone('JST'))`.
///
/// Handles the MAL card formats ("Apr 7, 2018", "Apr 7, 2018 12:00 AM") plus
/// bare digit strings (see [`parse_jst_digits`]). PHP throws for anything else
/// and the parsers catch it; `None` is the Rust equivalent.
fn parse_jst_datetime(date: &str) -> Option<DateTime<FixedOffset>> {
    let date = date.trim();
    if date.is_empty() || date.contains('?') {
        return None;
    }

    let jst = jst_offset();

    for format in ["%b %d, %Y", "%B %d, %Y", "%b %Y", "%B %Y"] {
        if let Ok(naive_date) = NaiveDate::parse_from_str(date, format) {
            let naive = naive_date.and_hms_opt(0, 0, 0)?;
            return jst.from_local_datetime(&naive).single();
        }
    }
    for format in ["%b %d, %Y %I:%M %p", "%B %d, %Y %I:%M %p"] {
        if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(date, format) {
            return jst.from_local_datetime(&naive).single();
        }
    }
    if let Ok(naive_date) = NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        let naive = naive_date.and_hms_opt(0, 0, 0)?;
        return jst.from_local_datetime(&naive).single();
    }
    if date.bytes().all(|byte| byte.is_ascii_digit()) {
        return parse_jst_digits(date);
    }
    None
}

/// `new \DateTimeImmutable($digits, JST)` for the bare digit strings PHP's
/// date parser accepts (the value handed over by
/// `MangaCardParser::getPublishDates()`):
///
/// - `HHMM` (4) / `HHMMSS` (6) are wall-clock times *today* (PHP's "today" is
///   the current date in JST), so `2009` means 20:09;
/// - other valid years (`1989`, `9999`, ...) are a year with the current
///   month/day/time;
/// - `YYYYMMDD` (8) is a date.
fn parse_jst_digits(digits: &str) -> Option<DateTime<FixedOffset>> {
    let jst = jst_offset();
    let now = Utc::now().with_timezone(&jst);
    let today = NaiveDate::from_ymd_opt(now.year(), now.month(), now.day())?;

    let time_of_day = |hour: u32, minute: u32, second: u32| -> Option<DateTime<FixedOffset>> {
        // PHP accepts hour 24 as the end of the day.
        if hour > 24 || minute > 59 || second > 59 {
            return None;
        }
        let naive = today.and_hms_opt(0, 0, 0)?
            + Duration::hours(hour as i64)
            + Duration::minutes(minute as i64)
            + Duration::seconds(second as i64);
        jst.from_local_datetime(&naive).single()
    };

    match digits.len() {
        4 => {
            let hour: u32 = digits[0..2].parse().ok()?;
            let minute: u32 = digits[2..4].parse().ok()?;
            if let Some(datetime) = time_of_day(hour, minute, 0) {
                return Some(datetime);
            }
            // Invalid HH:MM -> PHP treats the digits as a year.
            let year: i32 = digits.parse().ok()?;
            let naive = NaiveDate::from_ymd_opt(year, now.month(), now.day())?.and_hms_opt(
                now.hour(),
                now.minute(),
                now.second(),
            )?;
            jst.from_local_datetime(&naive).single()
        }
        6 => {
            let hour: u32 = digits[0..2].parse().ok()?;
            let minute: u32 = digits[2..4].parse().ok()?;
            let second: u32 = digits[4..6].parse().ok()?;
            time_of_day(hour, minute, second)
        }
        8 => {
            let year: i32 = digits[0..4].parse().ok()?;
            let month: u32 = digits[4..6].parse().ok()?;
            let day: u32 = digits[6..8].parse().ok()?;
            let naive = NaiveDate::from_ymd_opt(year, month, day)?.and_hms_opt(0, 0, 0)?;
            jst.from_local_datetime(&naive).single()
        }
        _ => None,
    }
}

/// `new \DateTimeZone('JST')` (`+09:00`, no DST).
fn jst_offset() -> FixedOffset {
    FixedOffset::east_opt(9 * 3600).expect("JST is a valid fixed offset")
}

/// PHP `(int) $string`: leading whitespace and sign, then digits; anything
/// non-numeric yields 0, overflow saturates.
fn php_intval(string: &str) -> i64 {
    let bytes = string.as_bytes();
    let mut i = 0;
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C) {
        i += 1;
    }
    let negative = if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        let negative = bytes[i] == b'-';
        i += 1;
        negative
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
    match string[start..i].parse::<i64>() {
        Ok(value) if negative => -value,
        Ok(value) => value,
        Err(_) if negative => i64::MIN,
        Err(_) => i64::MAX,
    }
}

/// PHP `(float) $string`: leading whitespace and sign, then the longest
/// numeric prefix (digits, optional fraction and exponent); non-numeric
/// prefixes yield 0.0.
fn php_floatval(string: &str) -> f64 {
    let bytes = string.as_bytes();
    let mut i = 0;
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C) {
        i += 1;
    }
    let start = i;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    let integer_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    let mut has_digits = i > integer_start;
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        let fraction_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        has_digits = has_digits || i > fraction_start;
    }
    if !has_digits {
        return 0.0;
    }
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        let mut exponent = i + 1;
        if exponent < bytes.len() && (bytes[exponent] == b'+' || bytes[exponent] == b'-') {
            exponent += 1;
        }
        let exponent_start = exponent;
        while exponent < bytes.len() && bytes[exponent].is_ascii_digit() {
            exponent += 1;
        }
        if exponent > exponent_start {
            i = exponent;
        }
    }
    string[start..i].parse::<f64>().unwrap_or(0.0)
}

fn item_meta_id_re() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(r"https://myanimelist\.net/\w+/(\d+).*").expect("valid regex")
    })
}

fn digits_re() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"([0-9]+)").expect("valid regex"))
}
