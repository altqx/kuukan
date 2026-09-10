//! Ports of `Jikan\Parser\Recommendations\*` (recent recommendations page).

use chrono::{DateTime, FixedOffset, Utc};
use regex::Regex;
use serde_json::{json, Value};
use std::sync::OnceLock;

use crate::error::ParseError;
use crate::parser::date::{format_atom, parse_date};
use crate::parser::helper::{parse_image_quality, HtmlDoc, HtmlNode};
use crate::parser::jstring::cleanse;
use crate::parser::mal_url::{id_from_url, BASE_URL};

/// `CommonImageResource::factory()`.
fn common_image_resource(image_url: Option<&str>) -> Value {
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

/// `Jikan\Model\Common\CommonMeta` constructor.
fn common_meta(title: &str, url: &str, image_url: &str) -> Value {
    let image = parse_image_quality(image_url);
    json!({
        "mal_id": id_from_url(url),
        "url": url,
        "images": common_image_resource(Some(&image)),
        "title": title,
    })
}

/// `Jikan\Parser\Recommendations\RecentRecommendationsParser`.
pub struct RecentRecommendationsParser<'a> {
    doc: &'a HtmlDoc,
}

impl<'a> RecentRecommendationsParser<'a> {
    pub fn new(doc: &'a HtmlDoc) -> Self {
        RecentRecommendationsParser { doc }
    }

    /// `RecentRecommendations::fromParser()`.
    pub fn get_model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "results": self.get_recent_recommendations()?,
            "has_next_page": self.has_next_page()?,
            "last_visible_page": self.get_last_page()?,
        }))
    }

    /// `RecentRecommendationsParser::getRecentRecommendations()`.
    pub fn get_recent_recommendations(&self) -> Result<Vec<Value>, ParseError> {
        let mut out = Vec::new();
        for node in self.doc.nodes(
            "//*[@id=\"content\"]/div[3]/div[contains(@class, \"spaceit borderClass\")]",
        )? {
            out.push(RecommendationListItemParser::new(&node).get_model()?);
        }
        Ok(out)
    }

    /// `RecentRecommendationsParser::getUserRecommendations()`.
    pub fn get_user_recommendations(&self) -> Result<Vec<Value>, ParseError> {
        let mut out = Vec::new();
        for node in self.doc.nodes(
            "//*[@id=\"content\"]/div/div[2]/div/div[2]/div[contains(@class, \"spaceit borderClass\")]",
        )? {
            out.push(RecommendationListItemParser::new(&node).get_model()?);
        }
        Ok(out)
    }

    /// `RecentRecommendationsParser::hasNextPage()`.
    pub fn has_next_page(&self) -> Result<bool, ParseError> {
        let Some(text) = self.doc.text("//*[@id=\"horiznav_nav\"]/div/span")? else {
            return Ok(false);
        };
        Ok(next_page_re().is_match(&text))
    }

    /// `RecentRecommendationsParser::getLastPage()`.
    pub fn get_last_page(&self) -> Result<i64, ParseError> {
        let Some(text) = self.doc.text("//*[@id=\"horiznav_nav\"]/div/span")? else {
            return Ok(1);
        };
        let last = text.split(' ').last().unwrap_or_default();
        Ok(last.replace(['[', ']'], "").parse().unwrap_or(1))
    }
}

/// `Jikan\Parser\Recommendations\RecommendationListItemParser`.
pub struct RecommendationListItemParser<'a> {
    node: &'a HtmlNode,
}

impl<'a> RecommendationListItemParser<'a> {
    pub fn new(node: &'a HtmlNode) -> Self {
        RecommendationListItemParser { node }
    }

    /// `RecommendationListItem::fromParser()`.
    pub fn get_model(&self) -> Result<Value, ParseError> {
        let entry = self.get_recommendations()?;
        let mal_id = format!(
            "{}-{}",
            entry.first().and_then(|e| e["mal_id"].as_i64()).unwrap_or(0),
            entry.get(1).and_then(|e| e["mal_id"].as_i64()).unwrap_or(0)
        );
        // PHP assigns `user` before `date`; `getDate()` removes the anchor
        // node, so the recommender must be read first.
        let user = self.get_recommender()?;
        let date = self.get_date()?;
        Ok(json!({
            "mal_id": mal_id,
            "entry": entry,
            "content": self.get_content()?,
            "date": date.map(|date| format_atom(&date)),
            "user": user,
        }))
    }

    /// `RecommendationListItemParser::getRecommendations()`.
    pub fn get_recommendations(&self) -> Result<Vec<Value>, ParseError> {
        let mut out = Vec::new();
        for node in self.node.nodes("//table/tr/td")? {
            out.push(common_meta(
                &node.text("//a/strong")?.unwrap_or_default(),
                &node.attr("//a", "href")?.unwrap_or_default(),
                &node.attr("//div[1]/a/img", "data-src")?.unwrap_or_default(),
            ));
        }
        Ok(out)
    }

    /// `RecommendationListItemParser::getContent()`.
    pub fn get_content(&self) -> Result<String, ParseError> {
        // User Profile Recommendations
        if let Some(node) = self
            .node
            .first("//p[contains(@class, \"profile-user-recs-text\")]")?
        {
            return Ok(node.node_text());
        }
        // Recent Recommendations
        Ok(self
            .node
            .text("//div[contains(@class, \"recommendations-user-recs-text\")]")?
            .unwrap_or_default())
    }

    /// `RecommendationListItemParser::getDate()`.
    ///
    /// PHP reads the date through `str_replace`-like `removeChildNodes`, then
    /// `preg_match('~- (.*)$~')`. If the regex does not match, `$time[1]` is
    /// null and `new DateTimeImmutable(null, UTC)` returns *now*.
    pub fn get_date(&self) -> Result<Option<DateTime<FixedOffset>>, ParseError> {
        let node = match self.node.first("//div[contains(@class, \"lightLink\")]")? {
            Some(node) => node,
            None => return Ok(None),
        };
        node.remove_child_nodes()?;
        let date = cleanse(&node.node_text());
        match date_re().captures(&date) {
            Some(caps) => Ok(parse_date(&caps[1])),
            None => Ok(Some(Utc::now().fixed_offset())),
        }
    }

    /// `RecommendationListItemParser::getRecommender()`.
    pub fn get_recommender(&self) -> Result<Value, ParseError> {
        let href = self
            .node
            .attr("//div[contains(@class, \"lightLink\")]/a", "href")?
            .unwrap_or_default();
        let username = self
            .node
            .text("//div[contains(@class, \"lightLink\")]/a")?
            .unwrap_or_default();
        Ok(json!({
            "url": format!("{BASE_URL}{href}"),
            "username": username,
        }))
    }
}

fn next_page_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\[\d+]\s(\d+)").expect("valid regex"))
}

fn date_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"- (.*)$").expect("valid regex"))
}
