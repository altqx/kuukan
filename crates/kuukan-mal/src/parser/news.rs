//! Ports of `Jikan\Parser\News\*`.
//!
//! [`parse_news`] is the shared entry point used by the anime/manga news
//! endpoints (`MalClient::getNewsList()`); it returns the JMS-shaped
//! `NewsListItem[]` payload.

use regex::Regex;
use serde_json::{json, Value};
use std::sync::OnceLock;

use crate::error::ParseError;
use crate::parser::date::{format_atom, parse_date};
use crate::parser::helper::{parse_image_quality, HtmlDoc, HtmlNode};
use crate::parser::jstring::cleanse;
use crate::parser::mal_url::MalUrlParser;

/// `Parser\News\NewsListParser::getResults()`.
///
/// Shared entry point for anime/manga news.
pub fn parse_news(doc: &HtmlDoc) -> Result<Vec<Value>, ParseError> {
    NewsListParser::new(doc).get_results()
}

/// `Jikan\Parser\News\NewsListParser`.
pub struct NewsListParser<'a> {
    doc: &'a HtmlDoc,
}

impl<'a> NewsListParser<'a> {
    pub fn new(doc: &'a HtmlDoc) -> Self {
        NewsListParser { doc }
    }

    /// `NewsListParser::getResults()`.
    pub fn get_results(&self) -> Result<Vec<Value>, ParseError> {
        let mut out = Vec::new();
        for node in self
            .doc
            .nodes("//div[contains(@class,\"js-scrollfix-bottom-rel\")]/div[@class=\"clearfix\"]")?
        {
            out.push(NewsListItemParser::new(&node).get_model()?);
        }
        Ok(out)
    }

    /// `NewsListParser::getHasNextPage()`.
    pub fn get_has_next_page(&self) -> Result<bool, ParseError> {
        Ok(self
            .doc
            .count("//*[@id=\"content\"]/table/tr/td[2]/div[1]/a[contains(text(), \"More News\")]")?
            > 0)
    }

    /// `NewsList::fromParser()`: `{results, has_next_page, last_visible_page}`.
    pub fn get_model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "results": self.get_results()?,
            "has_next_page": self.get_has_next_page()?,
            "last_visible_page": 1,
        }))
    }
}

/// `Jikan\Parser\News\NewsListItemParser`.
pub struct NewsListItemParser<'a> {
    node: &'a HtmlNode,
}

impl<'a> NewsListItemParser<'a> {
    pub fn new(node: &'a HtmlNode) -> Self {
        NewsListItemParser { node }
    }

    /// `NewsListItem::fromParser()`.
    pub fn get_model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "mal_id": self.get_mal_id()?,
            "url": self.get_url()?,
            "title": self.get_title()?,
            "date": self.get_date()?.map(|date| format_atom(&date)),
            "author_username": self.get_author()?.name().to_string(),
            "author_url": self.get_author()?.url().to_string(),
            "forum_url": self.get_discussion_link()?,
            "images": {
                "jpg": {
                    "image_url": self.get_image()?,
                },
            },
            "comments": self.get_comments()?,
            "excerpt": self.get_intro()?,
        }))
    }

    /// `NewsListItemParser::getTitle()`.
    pub fn get_title(&self) -> Result<String, ParseError> {
        Ok(self.node.text("//p/a/strong")?.unwrap_or_default())
    }

    /// `NewsListItemParser::getMalId()`.
    pub fn get_mal_id(&self) -> Result<Option<i64>, ParseError> {
        let url = self.get_url()?;
        Ok(mal_id_re()
            .captures(&url)
            .and_then(|caps| caps.get(1))
            .and_then(|m| m.as_str().parse().ok()))
    }

    /// `NewsListItemParser::getUrl()`.
    pub fn get_url(&self) -> Result<String, ParseError> {
        let href = self
            .node
            .first("//p/a/strong/..")?
            .and_then(|node| node.node_attr("href"))
            .unwrap_or_default();
        Ok(format!("{}{}", crate::parser::mal_url::BASE_URL, href))
    }

    /// `NewsListItemParser::getImage()`.
    pub fn get_image(&self) -> Result<Option<String>, ParseError> {
        match self.node.first("//img[1]")? {
            Some(image) => Ok(Some(parse_image_quality(
                image.node_attr("data-src").as_deref().unwrap_or_default(),
            ))),
            None => Ok(None),
        }
    }

    /// `NewsListItemParser::getDate()`.
    pub fn get_date(&self) -> Result<Option<chrono::DateTime<chrono::FixedOffset>>, ParseError> {
        let text = self.node.text("//p[last()]")?.unwrap_or_default();
        let date = text.split(" by").next().unwrap_or_default();
        Ok(parse_date(date))
    }

    /// `NewsListItemParser::getAuthor()`.
    pub fn get_author(&self) -> Result<crate::parser::mal_url::MalUrl, ParseError> {
        let node = self
            .node
            .first("//a[contains(@href, \"profile\")][1]")?
            .ok_or_else(|| ParseError::InvalidXPath("author link".to_string()))?;
        MalUrlParser::new(node).get_model()
    }

    /// `NewsListItemParser::getDiscussionLink()`.
    pub fn get_discussion_link(&self) -> Result<String, ParseError> {
        let href = self
            .node
            .first("//a[last()]")?
            .and_then(|node| node.node_attr("href"))
            .unwrap_or_default();
        Ok(format!("{}{}", crate::parser::mal_url::BASE_URL, href))
    }

    /// `NewsListItemParser::getComments()`.
    pub fn get_comments(&self) -> Result<i64, ParseError> {
        let text = self.node.text("//a[last()]")?.unwrap_or_default();
        Ok(comments_re()
            .captures(&text)
            .and_then(|caps| caps.get(1))
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(0))
    }

    /// `NewsListItemParser::getIntro()`.
    pub fn get_intro(&self) -> Result<String, ParseError> {
        let node = self.node.first("//p[2]")?;
        if let Some(node) = node {
            node.remove_child_nodes()?;
            Ok(cleanse(&node.node_text()))
        } else {
            Ok(String::new())
        }
    }
}

fn mal_id_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"([\d]+)$").expect("valid regex"))
}

fn comments_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"Discuss \((\d+) comments\)").expect("valid regex"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::helper::HtmlDoc;

    #[test]
    fn parses_news_item() {
        let doc = HtmlDoc::parse_str(
            r#"
            <div class="clearfix">
              <a href="/news/66547854"><img data-src="https://cdn.myanimelist.net/s/common/x/r/300x200.jpg?s=1" /></a>
              <p><a href="/news/66547854"><strong>Berserk resumes</strong></a></p>
              <p>Some excerpt <b>with</b> markup.</p>
              <p>Jun 7 by <a href="/profile/Vindstot">Vindstot</a></p>
              <a href="/forum/?topicid=2021160">Discuss (70 comments)</a>
            </div>
            "#,
        )
        .unwrap();
        let node = doc.first("//div").unwrap().unwrap();
        let model = NewsListItemParser::new(&node).get_model().unwrap();
        assert_eq!(model["mal_id"], 66547854);
        assert_eq!(model["url"], "https://myanimelist.net/news/66547854");
        assert_eq!(model["title"], "Berserk resumes");
        assert_eq!(model["author_username"], "Vindstot");
        assert_eq!(model["author_url"], "https://myanimelist.net/profile/Vindstot");
        assert_eq!(model["forum_url"], "https://myanimelist.net/forum/?topicid=2021160");
        assert_eq!(model["comments"], 70);
        assert_eq!(model["excerpt"], "Some excerpt with markup.");
        assert_eq!(
            model["images"]["jpg"]["image_url"],
            "https://cdn.myanimelist.net/s/common/x.jpg?s=1"
        );
        assert!(model["date"].is_string());
    }
}
