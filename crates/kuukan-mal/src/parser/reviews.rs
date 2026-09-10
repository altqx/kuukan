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
use crate::parser::helper::{parse_image_quality, parse_image_thumb_to_hq, HtmlDoc, HtmlNode};
use crate::parser::jstring::cleanse;
use crate::parser::mal_url::id_from_url;

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
    pub fn new(doc: &'a HtmlDoc) -> Self {
        ReviewsParser { doc }
    }

    /// `Reviews::fromParser()`: `{results, has_next_page, last_visible_page}`.
    pub fn get_model(&self) -> Result<JsonValue, ParseError> {
        Ok(json!({
            "results": self.get_reviews()?,
            "has_next_page": self.has_next_page()?,
            "last_visible_page": 1,
        }))
    }

    /// `ReviewsParser::getReviews()` (`array_filter` drops non-reviews).
    pub fn get_reviews(&self) -> Result<Vec<JsonValue>, ParseError> {
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
    pub fn has_next_page(&self) -> Result<bool, ParseError> {
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
    pub fn new(node: &'a HtmlNode) -> Self {
        ReviewerParser { node }
    }

    /// `Reviewer::fromParser()`: `{url, username, images}`.
    pub fn get_model(&self) -> Result<JsonValue, ParseError> {
        Ok(json!({
            "url": self.get_url()?,
            "username": self.get_username()?,
            "images": user_image_resource(&self.get_image_url()?),
        }))
    }

    /// `ReviewerParser::getUrl()`.
    pub fn get_url(&self) -> Result<String, ParseError> {
        // works on Anime/Manga Review pages
        if let Some(node) = self
            .node
            .first("//div/div[2]/div[contains(@class, \"username\")]/a")?
        {
            return Ok(node.node_attr("href").unwrap_or_default());
        }
        // works on Top UserReviewsParser pages, the div is shifted
        match self
            .node
            .first("//div[1]/div[1]/div[4]/table/tr/td[2]/a")?
        {
            Some(node) => Ok(node.node_attr("href").unwrap_or_default()),
            None => Err(ParseError::InvalidXPath(
                "Couldn't find any URL on review pages.".to_string(),
            )),
        }
    }

    /// `ReviewerParser::getUsername()`.
    pub fn get_username(&self) -> Result<String, ParseError> {
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
    pub fn get_image_url(&self) -> Result<String, ParseError> {
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
    pub fn new(node: &'a HtmlNode) -> Self {
        ReactionsParser { node }
    }

    /// `Reactions::fromParser()`.
    pub fn get_model(&self) -> Result<JsonValue, ParseError> {
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
        serde_json::from_str(&raw).unwrap_or_else(|_| {
            serde_json::from_str(DEFAULT_REACTIONS).expect("valid default")
        })
    }
}

/// PHP `(int)` for `serde_json::Value` scalars (numbers and numeric strings).
fn json_to_int(value: &JsonValue) -> Option<i64> {
    match value {
        JsonValue::Number(number) => number.as_i64().or_else(|| {
            number.as_f64().map(|float| float as i64)
        }),
        JsonValue::String(string) => Some(crate::parser::search::php_intval(string)),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Anime review
// ---------------------------------------------------------------------------

/// `Jikan\Parser\Reviews\AnimeReviewParser`.
pub struct AnimeReviewParser<'a> {
    node: &'a HtmlNode,
}

impl<'a> AnimeReviewParser<'a> {
    pub fn new(node: &'a HtmlNode) -> Self {
        AnimeReviewParser { node }
    }

    /// `FullAnimeReview::fromParser()`.
    pub fn get_model(&self) -> Result<JsonValue, ParseError> {
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
            "user": self.get_reviewer()?,
        }))
    }

    /// `AnimeReviewParser::getAnime()`.
    pub fn get_anime(&self) -> Result<JsonValue, ParseError> {
        Ok(item_meta(
            &self.get_anime_title()?,
            &self.get_anime_url()?,
            &self.get_anime_image_url()?,
        ))
    }

    /// `AnimeReviewParser::getId()`: `parse_str(parse_url($url, PHP_URL_QUERY))`.
    pub fn get_id(&self) -> Result<i64, ParseError> {
        let url = self.get_url()?;
        Ok(query_id_re()
            .captures(&url)
            .and_then(|caps| caps.get(1))
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(0))
    }

    /// `AnimeReviewParser::getUrl()`.
    pub fn get_url(&self) -> Result<String, ParseError> {
        Ok(self
            .node
            .attr(
                "//div/div[2]/div[contains(@class, \"bottom-navi\")]/div[@class=\"open\"]/a",
                "href",
            )?
            .unwrap_or_default())
    }

    /// `AnimeReviewParser::getAnimeTitle()`.
    pub fn get_anime_title(&self) -> Result<String, ParseError> {
        Ok(self
            .node
            .text("//div[contains(@class, \"titleblock\")]/a")?
            .unwrap_or_default())
    }

    /// `AnimeReviewParser::getAnimeUrl()`.
    pub fn get_anime_url(&self) -> Result<String, ParseError> {
        Ok(self
            .node
            .attr("//div[contains(@class, \"titleblock\")]/a", "href")?
            .unwrap_or_default())
    }

    /// `AnimeReviewParser::getAnimeImageUrl()`.
    pub fn get_anime_image_url(&self) -> Result<String, ParseError> {
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
    pub fn get_anime_image_url_from_user_page(&self) -> Result<String, ParseError> {
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
    pub fn get_date(&self) -> Result<Option<chrono::DateTime<chrono::FixedOffset>>, ParseError> {
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
    pub fn get_content(&self) -> Result<String, ParseError> {
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
    pub fn get_reviewer(&self) -> Result<JsonValue, ParseError> {
        ReviewerParser::new(self.node).get_model()
    }

    /// `AnimeReviewParser::getType()`.
    pub fn get_type(&self) -> Result<Option<String>, ParseError> {
        // Anime/Manga and User Reviews page
        if let Some(node) = self
            .node
            .first("//div/div/div[2]/div[2]/small")?
        {
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
    pub fn get_episodes_watched(&self) -> Result<Option<i64>, ParseError> {
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
    pub fn get_reactions(&self) -> Result<JsonValue, ParseError> {
        ReactionsParser::new(self.node).get_model()
    }

    /// `AnimeReviewParser::getReviewerScore()`.
    pub fn get_reviewer_score(&self) -> Result<i64, ParseError> {
        Ok(crate::parser::search::php_intval(
            &self
                .node
                .text("//div/div[2]/div[contains(@class, \"rating\")]/span")?
                .unwrap_or_default(),
        ))
    }

    /// `AnimeReviewParser::getReviewTag()`.
    pub fn get_review_tag(&self) -> Result<Vec<String>, ParseError> {
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
    pub fn is_preliminary(&self) -> Result<bool, ParseError> {
        Ok(self.node.count(
            "//div/div[2]/div[contains(@class, \"tags\")]/div[contains(@class, \"preliminary\")]",
        )? > 0)
    }

    /// `AnimeReviewParser::isSpoiler()`.
    pub fn is_spoiler(&self) -> Result<bool, ParseError> {
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
    pub fn new(node: &'a HtmlNode) -> Self {
        MangaReviewParser { node }
    }

    /// `FullMangaReview::fromParser()`.
    pub fn get_model(&self) -> Result<JsonValue, ParseError> {
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
            "user": self.get_reviewer()?,
        }))
    }

    /// `MangaReviewParser::getManga()`.
    pub fn get_manga(&self) -> Result<JsonValue, ParseError> {
        Ok(item_meta(
            &self.get_manga_title()?,
            &self.get_manga_url()?,
            &self.get_manga_image_url()?,
        ))
    }

    /// `MangaReviewParser::getId()`.
    pub fn get_id(&self) -> Result<i64, ParseError> {
        let url = self.get_url()?;
        Ok(query_id_re()
            .captures(&url)
            .and_then(|caps| caps.get(1))
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(0))
    }

    /// `MangaReviewParser::getUrl()`.
    pub fn get_url(&self) -> Result<String, ParseError> {
        Ok(self
            .node
            .attr(
                "//div/div[2]/div[contains(@class, \"bottom-navi\")]/div[@class=\"open\"]/a",
                "href",
            )?
            .unwrap_or_default())
    }

    /// `MangaReviewParser::getMangaTitle()`.
    pub fn get_manga_title(&self) -> Result<String, ParseError> {
        Ok(self
            .node
            .text("//div[contains(@class, \"titleblock\")]/a")?
            .unwrap_or_default())
    }

    /// `MangaReviewParser::getMangaUrl()`.
    pub fn get_manga_url(&self) -> Result<String, ParseError> {
        Ok(self
            .node
            .attr("//div[contains(@class, \"titleblock\")]/a", "href")?
            .unwrap_or_default())
    }

    /// `MangaReviewParser::getMangaImageUrl()`.
    pub fn get_manga_image_url(&self) -> Result<String, ParseError> {
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
    pub fn get_manga_image_url_from_user_page(&self) -> Result<String, ParseError> {
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
    pub fn get_date(&self) -> Result<Option<chrono::DateTime<chrono::FixedOffset>>, ParseError> {
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
    pub fn get_content(&self) -> Result<String, ParseError> {
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
    pub fn get_reviewer(&self) -> Result<JsonValue, ParseError> {
        ReviewerParser::new(self.node).get_model()
    }

    /// `MangaReviewParser::getType()`.
    pub fn get_type(&self) -> Result<Option<String>, ParseError> {
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
    pub fn get_chapters_read(&self) -> Result<Option<i64>, ParseError> {
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
    pub fn get_reactions(&self) -> Result<JsonValue, ParseError> {
        ReactionsParser::new(self.node).get_model()
    }

    /// `MangaReviewParser::getReviewerScore()`.
    pub fn get_reviewer_score(&self) -> Result<i64, ParseError> {
        Ok(crate::parser::search::php_intval(
            &self
                .node
                .text("//div/div[2]/div[contains(@class, \"rating\")]/span")?
                .unwrap_or_default(),
        ))
    }

    /// `MangaReviewParser::getReviewTag()`.
    pub fn get_review_tag(&self) -> Result<Vec<String>, ParseError> {
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
    pub fn is_preliminary(&self) -> Result<bool, ParseError> {
        Ok(self.node.count(
            "//div/div[2]/div[contains(@class, \"tags\")]/div[contains(@class, \"preliminary\")]",
        )? > 0)
    }

    /// `MangaReviewParser::isSpoiler()`.
    pub fn is_spoiler(&self) -> Result<bool, ParseError> {
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
