//! Ports of `Jikan\Parser\Reviews\*`.
//!
//! [`ReviewsParser`] is the global reviews page (`/reviews.php`); it yields
//! `FullAnimeReview`/`FullMangaReview` items. The per-item parsers
//! (`AnimeReviewParser`, `MangaReviewParser`, `ReviewerParser`,
//! `ReactionsParser`) mirror their PHP counterparts 1:1.

use regex::Regex;
use serde_json::{json, Value as JsonValue};
use std::sync::OnceLock;

use crate::error::ParseError;
use crate::parser::date::{format_atom, parse_date};
use crate::parser::helper::{HtmlDoc, HtmlNode};
use crate::parser::jstring::cleanse;
use crate::parser::mal_url::id_from_url;
use crate::parser::media_url::{parse_image_quality, parse_image_thumb_to_hq};

pub use crate::parser::mal_url::BASE_URL;

/// `CommonImageResource::factory()`.
fn common_image_resource(image_url: Option<&str>) -> JsonValue {
    match image_url {
        None => json!({
            "jpg": {"image_url": null, "small_image_url": null, "large_image_url": null},
            "webp": {"image_url": null, "small_image_url": null, "large_image_url": null},
        }),
        Some(url) => json!({
            "jpg": {
                "image_url": url,
                "small_image_url": url.replace(".jpg", "t.jpg"),
                "large_image_url": url.replace(".jpg", "l.jpg"),
            },
            "webp": {
                "image_url": url.replace(".jpg", ".webp"),
                "small_image_url": url.replace(".jpg", "t.webp"),
                "large_image_url": url.replace(".jpg", "l.webp"),
            },
        }),
    }
}

/// `UserImageResource::factory()`.
fn user_image_resource(image_url: &str) -> JsonValue {
    json!({
        "jpg": {"image_url": image_url},
        "webp": {"image_url": image_url.replace(".jpg", ".webp")},
    })
}

/// `Jikan\Model\Common\AnimeMeta` / `MangaMeta` constructor.
fn item_meta(title: &str, url: &str, image_url: &str) -> JsonValue {
    let image = parse_image_quality(image_url);
    json!({
        "mal_id": id_from_url(url),
        "url": url,
        "images": common_image_resource(Some(&image)),
        "title": title,
    })
}

// ---------------------------------------------------------------------------
// Global reviews page
// ---------------------------------------------------------------------------

/// `Jikan\Parser\Reviews\ReviewsParser`.
pub struct ReviewsParser<'a> {
    doc: &'a HtmlDoc,
}

impl<'a> ReviewsParser<'a> {
    pub(crate) fn new(doc: &'a HtmlDoc) -> Self {
        ReviewsParser { doc }
    }

    /// `Reviews::fromParser()`: `{results, has_next_page, last_visible_page}`.
    pub(crate) fn get_model(&self) -> Result<JsonValue, ParseError> {
        Ok(json!({
            "results": self.get_reviews()?,
            "has_next_page": self.has_next_page()?,
            "last_visible_page": 1,
        }))
    }

    /// `ReviewsParser::getReviews()` (`array_filter` drops non-reviews).
    fn get_reviews(&self) -> Result<Vec<JsonValue>, ParseError> {
        let mut out = Vec::new();
        for node in self
            .doc
            .nodes("//*[@id=\"content\"]//div[contains(@class, \"review-element\")]")?
        {
            let kind = node.text("//div/small")?.unwrap_or_default();
            if kind == "(Anime)" {
                out.push(AnimeReviewParser::new(&node).get_model()?);
            } else if kind == "(Manga)" {
                out.push(MangaReviewParser::new(&node).get_model()?);
            }
        }
        Ok(out)
    }

    /// `ReviewsParser::hasNextPage()`.
    fn has_next_page(&self) -> Result<bool, ParseError> {
        Ok(self
            .doc
            .count("//*[@id=\"horiznav_nav\"]/div/a[contains(text(), \"Next\")]")?
            > 0)
    }
}

// ---------------------------------------------------------------------------
// Reviewer
// ---------------------------------------------------------------------------

/// `Jikan\Parser\Reviews\ReviewerParser`.
pub struct ReviewerParser<'a> {
    node: &'a HtmlNode,
}

impl<'a> ReviewerParser<'a> {
    pub(crate) fn new(node: &'a HtmlNode) -> Self {
        ReviewerParser { node }
    }

    /// `Reviewer::fromParser()`: `{url, username, images}`.
    pub(crate) fn get_model(&self) -> Result<JsonValue, ParseError> {
        Ok(json!({
            "url": self.get_url()?,
            "username": self.get_username()?,
            "images": user_image_resource(&self.get_image_url()?),
        }))
    }

    /// `ReviewerParser::getUrl()`.
    pub(crate) fn get_url(&self) -> Result<String, ParseError> {
        // works on Anime/Manga Review pages
        if let Some(node) = self
            .node
            .first("//div/div[2]/div[contains(@class, \"username\")]/a")?
        {
            return Ok(node.node_attr("href").unwrap_or_default());
        }
        // works on Top UserReviewsParser pages, the div is shifted
        match self.node.first("//div[1]/div[1]/div[4]/table/tr/td[2]/a")? {
            Some(node) => Ok(node.node_attr("href").unwrap_or_default()),
            None => Err(ParseError::InvalidXPath(
                "Couldn't find any URL on review pages.".to_string(),
            )),
        }
    }

    /// `ReviewerParser::getUsername()`.
    fn get_username(&self) -> Result<String, ParseError> {
        // works on Anime/Manga Review pages
        if let Some(node) = self
            .node
            .first("//div/div[2]/div[contains(@class, \"username\")]/a")?
        {
            return Ok(node.node_text());
        }
        // works on Top UserReviewsParser pages, the div is shifted
        Ok(self
            .node
            .text("//div[1]/div[1]/div[4]/table/tr/td[2]/a")?
            .unwrap_or_default())
    }

    /// `ReviewerParser::getImageUrl()`.
    fn get_image_url(&self) -> Result<String, ParseError> {
        // works on Anime/Manga Review pages
        if let Some(node) = self.node.first("//div/div/a/img")? {
            return Ok(parse_image_thumb_to_hq(
                node.node_attr("data-src").as_deref().unwrap_or_default(),
            ));
        }
        // works on Top UserReviewsParser pages, the div is shifted
        let src = self
            .node
            .attr("//div[1]/div[1]/div[4]/table/tr/td[1]/div/a/img", "src")?
            .unwrap_or_default();
        Ok(parse_image_thumb_to_hq(&src))
    }
}

// ---------------------------------------------------------------------------
// Reactions
// ---------------------------------------------------------------------------

/// `{"icon":[],"num":0,"count":["0","0","0","0","0","0","0"]}`.
const DEFAULT_REACTIONS: &str = r#"{"icon":[],"num":0,"count":["0","0","0","0","0","0","0"]}"#;

/// `Jikan\Parser\Reviews\ReactionsParser`.
pub struct ReactionsParser<'a> {
    node: &'a HtmlNode,
}

impl<'a> ReactionsParser<'a> {
    pub(crate) fn new(node: &'a HtmlNode) -> Self {
        ReactionsParser { node }
    }

    /// `Reactions::fromParser()`.
    pub(crate) fn get_model(&self) -> Result<JsonValue, ParseError> {
        let reactions = self.reactions();
        let count = |index: usize| -> i64 {
            reactions
                .get("count")
                .and_then(|counts| counts.get(index))
                .and_then(json_to_int)
                .unwrap_or(0)
        };
        Ok(json!({
            "overall": reactions.get("num").and_then(json_to_int).unwrap_or(0),
            "nice": count(0),
            "love_it": count(1),
            "funny": count(2),
            "confusing": count(3),
            "informative": count(4),
            "well_written": count(5),
            "creative": count(6),
        }))
    }

    /// `ReactionsParser::getReactions()` + `json_decode(..., true)`.
    fn reactions(&self) -> JsonValue {
        let raw = cleanse(&self.node.node_attr("data-reactions").unwrap_or_default());
        if raw.is_empty() {
            return serde_json::from_str(DEFAULT_REACTIONS).expect("valid default");
        }
        serde_json::from_str(&raw)
            .unwrap_or_else(|_| serde_json::from_str(DEFAULT_REACTIONS).expect("valid default"))
    }
}

/// PHP `(int)` for `serde_json::Value` scalars (numbers and numeric strings).
fn json_to_int(value: &JsonValue) -> Option<i64> {
    match value {
        JsonValue::Number(number) => number
            .as_i64()
            .or_else(|| number.as_f64().map(|float| float as i64)),
        JsonValue::String(string) => Some(crate::parser::search::php_intval(string)),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Anime review
// ---------------------------------------------------------------------------

/// `Jikan\Parser\Reviews\AnimeReviewParser`.
/// The per-category score breakdown MAL renders beside a review.
///
/// Ported from `MangaReviewScoresParser`, which jikan-php never called, so the
/// breakdown never reached a response. It is shared by anime and manga reviews
/// here because the markup is the same table either way.
///
/// Every score is optional and the whole block is `None` when MAL renders no
/// table: a missing breakdown must read as absent, not as five zeros.
pub(crate) struct ReviewScoresParser<'a> {
    node: &'a HtmlNode,
}

impl<'a> ReviewScoresParser<'a> {
    pub(crate) fn new(node: &'a HtmlNode) -> Self {
        ReviewScoresParser { node }
    }

    fn row(&self, xpath: &str) -> Result<Option<i64>, ParseError> {
        Ok(self
            .node
            .text(xpath)?
            .map(|text| score_int(&text))
            .unwrap_or(None))
    }

    /// `None` when this review carries no breakdown table.
    pub(crate) fn model(&self) -> Result<Option<JsonValue>, ParseError> {
        let overall = self.row("//table/tr[1]/td[2]/strong")?;
        let story = self.row("//table/tr[2]/td[2]")?;
        let art = self.row("//table/tr[3]/td[2]")?;
        let character = self.row("//table/tr[4]/td[2]")?;
        let enjoyment = self.row("//table/tr[5]/td[2]")?;
        if [overall, story, art, character, enjoyment]
            .iter()
            .all(Option::is_none)
        {
            return Ok(None);
        }
        Ok(Some(json!({
            "overall": overall,
            "story": story,
            "art": art,
            "character": character,
            "enjoyment": enjoyment,
        })))
    }
}

/// Leading-digit cast, but `None` rather than `0` when there are no digits, so
/// an empty cell is absent instead of a real score of zero.
fn score_int(input: &str) -> Option<i64> {
    let digits: String = input
        .trim_start()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

pub struct AnimeReviewParser<'a> {
    node: &'a HtmlNode,
}

impl<'a> AnimeReviewParser<'a> {
    pub(crate) fn new(node: &'a HtmlNode) -> Self {
        AnimeReviewParser { node }
    }

    /// `FullAnimeReview::fromParser()`.
    pub(crate) fn get_model(&self) -> Result<JsonValue, ParseError> {
        Ok(json!({
            "mal_id": self.get_id()?,
            "url": self.get_url()?,
            "type": self.get_type()?,
            "reactions": self.get_reactions()?,
            "date": self.get_date()?.map(|date| format_atom(&date)),
            "review": self.get_content()?,
            "score": self.get_reviewer_score()?,
            "tags": self.get_review_tag()?,
            "is_spoiler": self.is_spoiler()?,
            "is_preliminary": self.is_preliminary()?,
            "episodes_watched": self.get_episodes_watched()?,
            "entry": item_meta(
                &self.get_anime_title()?,
                &self.get_anime_url()?,
                &self.get_anime_image_url()?,
            ),
            "scores": ReviewScoresParser::new(self.node).model()?,
            "user": self.get_reviewer()?,
        }))
    }

    /// `AnimeReviewParser::getId()`: `parse_str(parse_url($url, PHP_URL_QUERY))`.
    pub(crate) fn get_id(&self) -> Result<i64, ParseError> {
        let url = self.get_url()?;
        Ok(query_id_re()
            .captures(&url)
            .and_then(|caps| caps.get(1))
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(0))
    }

    /// `AnimeReviewParser::getUrl()`.
    pub(crate) fn get_url(&self) -> Result<String, ParseError> {
        Ok(self
            .node
            .attr(
                "//div/div[2]/div[contains(@class, \"bottom-navi\")]/div[@class=\"open\"]/a",
                "href",
            )?
            .unwrap_or_default())
    }

    /// `AnimeReviewParser::getAnimeTitle()`.
    pub(crate) fn get_anime_title(&self) -> Result<String, ParseError> {
        Ok(self
            .node
            .text("//div[contains(@class, \"titleblock\")]/a")?
            .unwrap_or_default())
    }

    /// `AnimeReviewParser::getAnimeUrl()`.
    pub(crate) fn get_anime_url(&self) -> Result<String, ParseError> {
        Ok(self
            .node
            .attr("//div[contains(@class, \"titleblock\")]/a", "href")?
            .unwrap_or_default())
    }

    /// `AnimeReviewParser::getAnimeImageUrl()`.
    fn get_anime_image_url(&self) -> Result<String, ParseError> {
        let src = self
            .node
            .attr(
                "//div[contains(@class, \"thumbbody\")]/div[contains(@class, \"body\")]/div[contains(@class, \"text\")]/div[contains(@class, \"thumb-right\")]/a/img",
                "data-src",
            )?
            .unwrap_or_default();
        Ok(parse_image_quality(&src))
    }

    /// `AnimeReviewParser::getAnimeImageUrlFromUserPage()`.
    pub(crate) fn get_anime_image_url_from_user_page(&self) -> Result<String, ParseError> {
        let src = self
            .node
            .attr(
                "//div[contains(@class, \"thumbbody\")]/div[contains(@class, \"thumb\")]/a/img",
                "data-src",
            )?
            .unwrap_or_default();
        Ok(parse_image_quality(&src))
    }

    /// `AnimeReviewParser::getDate()`.
    pub(crate) fn get_date(
        &self,
    ) -> Result<Option<chrono::DateTime<chrono::FixedOffset>>, ParseError> {
        let Some(node) = self
            .node
            .first("//div/div[2]/div[contains(@class, \"update_at\")]")?
        else {
            return Ok(None);
        };
        let date = node.node_text();
        let time = node.node_attr("title").unwrap_or_default();
        Ok(parse_date(&format!("{date} {time}")))
    }

    /// `AnimeReviewParser::getContent()`.
    pub(crate) fn get_content(&self) -> Result<String, ParseError> {
        let expanded = self.node.first(
            "//div/div[2]/div[contains(@class, \"text\")]/span[contains(@class, \"js-hidden\")]",
        )?;
        let node = self
            .node
            .first("//div/div[2]/div[contains(@class, \"text\")]")?;

        let mut content = match node {
            Some(node) => {
                node.remove_child_nodes()?;
                cleanse(&node.node_text())
            }
            None => String::new(),
        };

        if let Some(expanded) = expanded {
            expanded.remove_child_nodes()?;
            content.push_str(&cleanse(&expanded.node_html()));
        }

        Ok(content)
    }

    /// `AnimeReviewParser::getReviewer()`.
    pub(crate) fn get_reviewer(&self) -> Result<JsonValue, ParseError> {
        ReviewerParser::new(self.node).get_model()
    }

    /// `AnimeReviewParser::getType()`.
    pub(crate) fn get_type(&self) -> Result<Option<String>, ParseError> {
        // Anime/Manga and User Reviews page
        if let Some(node) = self.node.first("//div/div/div[2]/div[2]/small")? {
            return Ok(Some(
                node.node_text()
                    .replace(['(', ')'], "")
                    .to_ascii_lowercase(),
            ));
        }
        // All Reviews Page
        if let Some(node) = self.node.first("//div/small")? {
            return Ok(Some(
                node.node_text()
                    .replace(['(', ')'], "")
                    .to_ascii_lowercase(),
            ));
        }
        Ok(None)
    }

    /// `AnimeReviewParser::getEpisodesWatched()`.
    pub(crate) fn get_episodes_watched(&self) -> Result<Option<i64>, ParseError> {
        let Some(node) = self.node.first(
            "//div/div[2]/div[contains(@class, \"tags\")]/div[contains(@class, \"preliminary\")]/span",
        )? else {
            return Ok(None);
        };
        let text = cleanse(&node.node_text());
        Ok(Some(match seen_re().captures(&text) {
            Some(caps) => caps[1].parse().unwrap_or(0),
            None => 0,
        }))
    }

    /// `AnimeReviewParser::getReactions()`.
    pub(crate) fn get_reactions(&self) -> Result<JsonValue, ParseError> {
        ReactionsParser::new(self.node).get_model()
    }

    /// `AnimeReviewParser::getReviewerScore()`.
    pub(crate) fn get_reviewer_score(&self) -> Result<i64, ParseError> {
        Ok(crate::parser::search::php_intval(
            &self
                .node
                .text("//div/div[2]/div[contains(@class, \"rating\")]/span")?
                .unwrap_or_default(),
        ))
    }

    /// `AnimeReviewParser::getReviewTag()`.
    pub(crate) fn get_review_tag(&self) -> Result<Vec<String>, ParseError> {
        let mut out = Vec::new();
        for node in self
            .node
            .nodes("//div/div[2]/div[contains(@class, \"tags\")]/div")?
        {
            node.remove_child_nodes()?;
            out.push(cleanse(&node.node_text()));
        }
        Ok(out)
    }

    /// `AnimeReviewParser::isPreliminary()`.
    pub(crate) fn is_preliminary(&self) -> Result<bool, ParseError> {
        Ok(self.node.count(
            "//div/div[2]/div[contains(@class, \"tags\")]/div[contains(@class, \"preliminary\")]",
        )? > 0)
    }

    /// `AnimeReviewParser::isSpoiler()`.
    pub(crate) fn is_spoiler(&self) -> Result<bool, ParseError> {
        Ok(self.node.count(
            "//div/div[2]/div[contains(@class, \"tags\")]/div[contains(@class, \"spoiler\")]",
        )? > 0)
    }
}

// ---------------------------------------------------------------------------
// Manga review
// ---------------------------------------------------------------------------

/// `Jikan\Parser\Reviews\MangaReviewParser`.
pub struct MangaReviewParser<'a> {
    node: &'a HtmlNode,
}

impl<'a> MangaReviewParser<'a> {
    pub(crate) fn new(node: &'a HtmlNode) -> Self {
        MangaReviewParser { node }
    }

    /// `FullMangaReview::fromParser()`.
    pub(crate) fn get_model(&self) -> Result<JsonValue, ParseError> {
        Ok(json!({
            "mal_id": self.get_id()?,
            "url": self.get_url()?,
            "type": self.get_type()?.unwrap_or_else(|| "manga".to_string()),
            "reactions": self.get_reactions()?,
            "date": self.get_date()?.map(|date| format_atom(&date)),
            "review": self.get_content()?,
            "score": self.get_reviewer_score()?,
            "tags": self.get_review_tag()?,
            "is_spoiler": self.is_spoiler()?,
            "is_preliminary": self.is_preliminary()?,
            "chapters_read": self.get_chapters_read()?,
            "entry": item_meta(
                &self.get_manga_title()?,
                &self.get_manga_url()?,
                &self.get_manga_image_url()?,
            ),
            "scores": ReviewScoresParser::new(self.node).model()?,
            "user": self.get_reviewer()?,
        }))
    }

    /// `MangaReviewParser::getId()`.
    pub(crate) fn get_id(&self) -> Result<i64, ParseError> {
        let url = self.get_url()?;
        Ok(query_id_re()
            .captures(&url)
            .and_then(|caps| caps.get(1))
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(0))
    }

    /// `MangaReviewParser::getUrl()`.
    pub(crate) fn get_url(&self) -> Result<String, ParseError> {
        Ok(self
            .node
            .attr(
                "//div/div[2]/div[contains(@class, \"bottom-navi\")]/div[@class=\"open\"]/a",
                "href",
            )?
            .unwrap_or_default())
    }

    /// `MangaReviewParser::getMangaTitle()`.
    pub(crate) fn get_manga_title(&self) -> Result<String, ParseError> {
        Ok(self
            .node
            .text("//div[contains(@class, \"titleblock\")]/a")?
            .unwrap_or_default())
    }

    /// `MangaReviewParser::getMangaUrl()`.
    pub(crate) fn get_manga_url(&self) -> Result<String, ParseError> {
        Ok(self
            .node
            .attr("//div[contains(@class, \"titleblock\")]/a", "href")?
            .unwrap_or_default())
    }

    /// `MangaReviewParser::getMangaImageUrl()`.
    fn get_manga_image_url(&self) -> Result<String, ParseError> {
        let src = self
            .node
            .attr(
                "//div[contains(@class, \"thumbbody\")]/div[contains(@class, \"body\")]/div[contains(@class, \"text\")]/div[contains(@class, \"thumb-right\")]/a/img",
                "data-src",
            )?
            .unwrap_or_default();
        Ok(parse_image_quality(&src))
    }

    /// `MangaReviewParser::getMangaImageUrlFromUserPage()`.
    pub(crate) fn get_manga_image_url_from_user_page(&self) -> Result<String, ParseError> {
        let src = self
            .node
            .attr(
                "//div[contains(@class, \"thumbbody\")]/div[contains(@class, \"thumb\")]/a/img",
                "data-src",
            )?
            .unwrap_or_default();
        Ok(parse_image_quality(&src))
    }

    /// `MangaReviewParser::getDate()`.
    pub(crate) fn get_date(
        &self,
    ) -> Result<Option<chrono::DateTime<chrono::FixedOffset>>, ParseError> {
        let Some(node) = self
            .node
            .first("//div/div[2]/div[contains(@class, \"update_at\")]")?
        else {
            return Ok(None);
        };
        let date = node.node_text();
        let time = node.node_attr("title").unwrap_or_default();
        Ok(parse_date(&format!("{date} {time}")))
    }

    /// `MangaReviewParser::getContent()`.
    pub(crate) fn get_content(&self) -> Result<String, ParseError> {
        let expanded = self.node.first(
            "//div/div[2]/div[contains(@class, \"text\")]/span[contains(@class, \"js-hidden\")]",
        )?;
        let node = self
            .node
            .first("//div/div[2]/div[contains(@class, \"text\")]")?;

        let mut content = match node {
            Some(node) => {
                node.remove_child_nodes()?;
                cleanse(&node.node_text())
            }
            None => String::new(),
        };

        if let Some(expanded) = expanded {
            expanded.remove_child_nodes()?;
            content.push_str(&cleanse(&expanded.node_html()));
        }

        Ok(content)
    }

    /// `MangaReviewParser::getReviewer()`.
    pub(crate) fn get_reviewer(&self) -> Result<JsonValue, ParseError> {
        ReviewerParser::new(self.node).get_model()
    }

    /// `MangaReviewParser::getType()`.
    pub(crate) fn get_type(&self) -> Result<Option<String>, ParseError> {
        if let Some(node) = self.node.first("//div/div/div[2]/div[2]/small")? {
            return Ok(Some(
                node.node_text()
                    .replace(['(', ')'], "")
                    .to_ascii_lowercase(),
            ));
        }
        if let Some(node) = self.node.first("//div/small")? {
            return Ok(Some(
                node.node_text()
                    .replace(['(', ')'], "")
                    .to_ascii_lowercase(),
            ));
        }
        Ok(None)
    }

    /// `MangaReviewParser::getChaptersRead()`.
    pub(crate) fn get_chapters_read(&self) -> Result<Option<i64>, ParseError> {
        let Some(node) = self.node.first(
            "//div/div[2]/div[contains(@class, \"tags\")]/div[contains(@class, \"preliminary\")]/span",
        )? else {
            return Ok(None);
        };
        let text = cleanse(&node.node_text());
        Ok(Some(match seen_re().captures(&text) {
            Some(caps) => caps[1].parse().unwrap_or(0),
            None => 0,
        }))
    }

    /// `MangaReviewParser::getReactions()`.
    pub(crate) fn get_reactions(&self) -> Result<JsonValue, ParseError> {
        ReactionsParser::new(self.node).get_model()
    }

    /// `MangaReviewParser::getReviewerScore()`.
    pub(crate) fn get_reviewer_score(&self) -> Result<i64, ParseError> {
        Ok(crate::parser::search::php_intval(
            &self
                .node
                .text("//div/div[2]/div[contains(@class, \"rating\")]/span")?
                .unwrap_or_default(),
        ))
    }

    /// `MangaReviewParser::getReviewTag()`.
    pub(crate) fn get_review_tag(&self) -> Result<Vec<String>, ParseError> {
        let mut out = Vec::new();
        for node in self
            .node
            .nodes("//div/div[2]/div[contains(@class, \"tags\")]/div")?
        {
            node.remove_child_nodes()?;
            out.push(cleanse(&node.node_text()));
        }
        Ok(out)
    }

    /// `MangaReviewParser::isPreliminary()`.
    pub(crate) fn is_preliminary(&self) -> Result<bool, ParseError> {
        Ok(self.node.count(
            "//div/div[2]/div[contains(@class, \"tags\")]/div[contains(@class, \"preliminary\")]",
        )? > 0)
    }

    /// `MangaReviewParser::isSpoiler()`.
    pub(crate) fn is_spoiler(&self) -> Result<bool, ParseError> {
        Ok(self.node.count(
            "//div/div[2]/div[contains(@class, \"tags\")]/div[contains(@class, \"spoiler\")]",
        )? > 0)
    }
}

fn query_id_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[?&]id=(\d+)").expect("valid regex"))
}

fn seen_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\((\d+)/(.*)\)").expect("valid regex"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A review with MAL's breakdown table yields every category.
    #[test]
    fn review_scores_read_the_breakdown_table() {
        let doc = HtmlDoc::parse_str(
            r#"<div class="review-element"><table>
                 <tr><td>Overall</td><td><strong>9</strong></td></tr>
                 <tr><td>Story</td><td>8</td></tr>
                 <tr><td>Art</td><td>10</td></tr>
                 <tr><td>Character</td><td>7</td></tr>
                 <tr><td>Enjoyment</td><td>9</td></tr>
               </table></div>"#,
        )
        .expect("doc");
        let node = doc
            .first("//div[@class='review-element']")
            .expect("xpath")
            .expect("node");
        let scores = ReviewScoresParser::new(&node)
            .model()
            .expect("model")
            .expect("a table is present");
        assert_eq!(scores["overall"], 9);
        assert_eq!(scores["story"], 8);
        assert_eq!(scores["art"], 10);
        assert_eq!(scores["character"], 7);
        assert_eq!(scores["enjoyment"], 9);
    }

    /// The case that matters: MAL renders no breakdown for many reviews, and
    /// that has to read as absent rather than as a real score of zero.
    #[test]
    fn review_scores_are_absent_without_a_table() {
        let doc = HtmlDoc::parse_str(r#"<div class="review-element"><p>no table</p></div>"#)
            .expect("doc");
        let node = doc
            .first("//div[@class='review-element']")
            .expect("xpath")
            .expect("node");
        assert!(ReviewScoresParser::new(&node)
            .model()
            .expect("model")
            .is_none());
    }

    /// A partially filled table keeps the categories MAL did render and nulls
    /// the rest.
    #[test]
    fn review_scores_null_the_missing_categories() {
        let doc = HtmlDoc::parse_str(
            r#"<div class="review-element"><table>
                 <tr><td>Overall</td><td><strong>6</strong></td></tr>
               </table></div>"#,
        )
        .expect("doc");
        let node = doc
            .first("//div[@class='review-element']")
            .expect("xpath")
            .expect("node");
        let scores = ReviewScoresParser::new(&node)
            .model()
            .expect("model")
            .expect("overall is present");
        assert_eq!(scores["overall"], 6);
        assert!(scores["story"].is_null());
        assert!(scores["enjoyment"].is_null());
    }
}
