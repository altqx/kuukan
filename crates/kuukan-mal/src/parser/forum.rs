//! Ports of `Jikan\Parser\Forum\*`.
//!
//! [`parse_forum`] is the shared entry point used by the anime/manga forum
//! endpoints (`MalClient::getAnimeForum()` / `getMangaForum()`); it returns the
//! JMS-shaped `ForumTopic[]` payload.

use serde_json::{json, Value};

use crate::error::ParseError;
use crate::parser::date::{format_atom, parse_date, parse_forum_date};
use crate::parser::helper::{HtmlDoc, HtmlNode};
use crate::parser::jstring::cleanse;
use crate::parser::mal_url::BASE_URL;

/// `Parser\Forum\ForumPageParser::getTopics()`.
///
/// Shared entry point for anime/manga forum.
pub fn parse_forum(doc: &HtmlDoc) -> Result<Vec<Value>, ParseError> {
    ForumPageParser::new(doc).get_topics()
}

/// `Jikan\Parser\Forum\ForumPageParser`.
pub struct ForumPageParser<'a> {
    doc: &'a HtmlDoc,
}

impl<'a> ForumPageParser<'a> {
    pub fn new(doc: &'a HtmlDoc) -> Self {
        ForumPageParser { doc }
    }

    /// `ForumPageParser::getTopics()`.
    pub fn get_topics(&self) -> Result<Vec<Value>, ParseError> {
        let mut out = Vec::new();
        for node in self.doc.nodes("//tr[contains(@id, \"topicRow\")]")? {
            out.push(ForumTopicParser::new(&node).get_model()?);
        }
        Ok(out)
    }
}

/// `Jikan\Parser\Forum\ForumTopicParser`.
pub struct ForumTopicParser<'a> {
    node: &'a HtmlNode,
}

impl<'a> ForumTopicParser<'a> {
    pub fn new(node: &'a HtmlNode) -> Self {
        ForumTopicParser { node }
    }

    /// `ForumTopic::fromParser()`.
    pub fn get_model(&self) -> Result<Value, ParseError> {
        let last_post = self.get_last_post()?;
        Ok(json!({
            "mal_id": self.get_topic_id()?,
            "url": self.get_url()?,
            "title": self.get_title()?,
            "date": self.get_post_date()?.map(|date| format_atom(&date)),
            "author_username": self.get_author_name()?,
            "author_url": self.get_author_url()?,
            "comments": self.get_replies()?,
            "last_comment": {
                "url": last_post.url,
                "author_username": last_post.author_username,
                "author_url": last_post.author_url,
                "date": last_post.date.map(|date| format_atom(&date)),
            },
        }))
    }

    /// `ForumTopicParser::getTopicId()`.
    pub fn get_topic_id(&self) -> Result<i64, ParseError> {
        let url = self.get_url()?;
        Ok(parse_str_topic_id(&url))
    }

    /// `ForumTopicParser::getUrl()`.
    pub fn get_url(&self) -> Result<String, ParseError> {
        let href = self
            .node
            .first("//a[2]")?
            .and_then(|node| node.node_attr("href"))
            .unwrap_or_default();
        Ok(format!("{BASE_URL}{href}"))
    }

    /// `ForumTopicParser::getTitle()`.
    pub fn get_title(&self) -> Result<String, ParseError> {
        Ok(self.node.text("//a[2]")?.unwrap_or_default())
    }

    /// `ForumTopicParser::getPostDate()`.
    pub fn get_post_date(
        &self,
    ) -> Result<Option<chrono::DateTime<chrono::FixedOffset>>, ParseError> {
        let text = self
            .node
            .text("//td[2]/span[@class=\"lightLink\"]")?
            .unwrap_or_default();
        Ok(parse_forum_date(&text))
    }

    /// `ForumTopicParser::getAuthorName()`.
    pub fn get_author_name(&self) -> Result<String, ParseError> {
        Ok(self
            .node
            .text("//span[@class=\"forum_postusername\"]/a")?
            .unwrap_or_default())
    }

    /// `ForumTopicParser::getAuthorUrl()`.
    pub fn get_author_url(&self) -> Result<String, ParseError> {
        let href = self
            .node
            .first("//span[@class=\"forum_postusername\"]/a")?
            .and_then(|node| node.node_attr("href"))
            .unwrap_or_default();
        Ok(format!("{BASE_URL}{href}"))
    }

    /// `ForumTopicParser::getReplies()`.
    pub fn get_replies(&self) -> Result<i64, ParseError> {
        let text = self.node.text("//td[3]")?.unwrap_or_default();
        Ok(php_intval(&text))
    }

    /// `ForumTopicParser::getLastPost()`.
    pub fn get_last_post(&self) -> Result<ForumPost, ParseError> {
        let author_name = self.node.text("//td[4]/a[1]")?.unwrap_or_default();
        let author_href = self
            .node
            .first("//td[4]/a[1]")?
            .and_then(|node| node.node_attr("href"))
            .unwrap_or_default();
        let url_href = self
            .node
            .first("//td[4]/a[2]")?
            .and_then(|node| node.node_attr("href"))
            .unwrap_or_default();

        let date = match self.node.first("//td[4]")? {
            Some(td) => {
                td.remove_child_nodes()?;
                let text = cleanse(&td.node_text());
                parse_date(&text.replace("by ", ""))
            }
            None => None,
        };

        Ok(ForumPost {
            url: format!("{BASE_URL}{url_href}"),
            author_username: author_name,
            author_url: format!("{BASE_URL}{author_href}"),
            date,
        })
    }
}

/// `Jikan\Model\Forum\ForumPost`.
pub struct ForumPost {
    pub url: String,
    pub author_username: String,
    pub author_url: String,
    pub date: Option<chrono::DateTime<chrono::FixedOffset>>,
}

/// `parse_str(explode('?', $url)[1], $query)['topicid']`.
fn parse_str_topic_id(url: &str) -> i64 {
    let query = url.split_once('?').map(|(_, q)| q).unwrap_or_default();
    for pair in query.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            if key == "topicid" {
                return php_intval(value);
            }
        }
    }
    0
}

/// PHP `(int) $string`.
fn php_intval(s: &str) -> i64 {
    let trimmed = s.trim_start();
    let mut chars = trimmed.chars().peekable();
    let negative = matches!(chars.peek(), Some('-') | Some('+'));
    if negative {
        let sign = chars.next();
        let sign = if sign == Some('-') { -1 } else { 1 };
        return sign * parse_leading_digits(&chars.collect::<String>());
    }
    parse_leading_digits(trimmed)
}

fn parse_leading_digits(s: &str) -> i64 {
    let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::helper::HtmlDoc;

    #[test]
    fn parses_forum_topic() {
        let doc = HtmlDoc::parse_str(
            r#"
            <table>
              <tr id="topicRow1">
                <td><a href="/login.php?from=x">watch</a></td>
                <td><a href="/forum/?topicid=1">First</a><span class="lightLink">May 14, 2008</span></td>
                <td>5</td>
                <td nowrap>by <a href="/profile/A">A</a> <a href="/forum/?topicid=1&goto=lastpost">&raquo;&raquo;</a><br>May 14, 2008</td>
              </tr>
              <tr id="topicRow2">
                <td><a href="/login.php?from=x">watch</a></td>
                <td>
                  <a href="/forum/?topicid=24885">Cowboy Bebop Episode 18 Discussion</a>
                  <span class="forum_postusername"><a href="/profile/FighterZ">FighterZ</a></span> -
                  <span class="lightLink">May 14, 2008</span>
                </td>
                <td>160</td>
                <td nowrap>by <a href="/profile/Daiko">Daiko</a> <a href="/forum/?topicid=24885&goto=lastpost">&raquo;&raquo;</a><br>Sep 28, 9:21 PM</td>
              </tr>
            </table>
            "#,
        )
        .unwrap();
        let topics = parse_forum(&doc).unwrap();
        assert_eq!(topics.len(), 2);
        assert_eq!(topics[1]["mal_id"], 24885);
        assert_eq!(topics[1]["title"], "Cowboy Bebop Episode 18 Discussion");
        assert_eq!(topics[1]["comments"], 160);
        assert_eq!(topics[1]["author_username"], "FighterZ");
        assert_eq!(
            topics[1]["last_comment"]["url"],
            "https://myanimelist.net/forum/?topicid=24885&goto=lastpost"
        );
        assert_eq!(topics[1]["last_comment"]["author_username"], "Daiko");
    }
}
