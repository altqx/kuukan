//! Port of `Jikan\Parser\Magazine\*` (jikan-php v4.0.12).

use regex::Regex;
use serde_json::{json, Value};
use std::sync::OnceLock;

use crate::error::ParseError;
use crate::parser::common::manga_card;
use crate::parser::helper::HtmlDoc;
use crate::parser::jstring::cleanse;
use crate::parser::mal_url::MalUrl;

/// `Jikan\Parser\Magazine\MagazineParser`.
pub struct MagazineParser {
    doc: HtmlDoc,
}

impl MagazineParser {
    pub fn new(doc: HtmlDoc) -> Self {
        MagazineParser { doc }
    }

    /// `MagazineParser::getResults()`.
    pub fn results(&self) -> Result<Vec<Value>, ParseError> {
        self.doc
            .css_nodes("div.seasonal-anime")?
            .iter()
            .map(manga_card)
            .collect()
    }

    /// `MagazineParser::getUrl()`.
    pub fn url(&self) -> Result<Value, ParseError> {
        let name = match self.doc.first("//span[@class='di-ib mt4']")? {
            Some(span) => {
                span.remove_child_nodes()?;
                cleanse(&span.node_text())
            }
            None => String::new(),
        };
        let href = self
            .doc
            .attr("//meta[@property=\"og:url\"]", "content")?
            .unwrap_or_default();
        Ok(MalUrl::new(name, href).to_json())
    }

    /// `MagazineParser::getLastPage()`.
    pub fn last_page(&self) -> Result<i64, ParseError> {
        let links = self
            .doc
            .nodes("//*[@id=\"content\"]/div[5]/div/a[contains(@class, \"link\")]")?;
        let Some(first) = links.first() else {
            return Ok(1);
        };
        let Some(last) = first.next_all().pop() else {
            return Ok(1);
        };
        let href = last.node_attr("href").unwrap_or_default();
        Ok(page_re()
            .captures(&href)
            .and_then(|caps| caps[1].parse().ok())
            .unwrap_or(1))
    }

    /// `MagazineParser::getHasNextPage()`.
    pub fn has_next_page(&self) -> Result<bool, ParseError> {
        let links = self
            .doc
            .nodes("//*[@id=\"content\"]/div[5]/div/a[contains(@class, \"link\")]")?;
        let Some(first) = links.first() else {
            return Ok(false);
        };
        let Some(last) = first.next_all().pop() else {
            return Ok(false);
        };
        Ok(!last
            .nodes("//a[not(contains(@class, \"current\"))]")?
            .is_empty())
    }

    /// `MagazineParser::getModel()`.
    pub fn model(&self) -> Result<Value, ParseError> {
        let url = self.url()?;
        Ok(json!({
            "results": self.results()?,
            "mal_id": url["mal_id"],
            "url": url["url"],
            "name": url["name"],
            "has_next_page": self.has_next_page()?,
            "last_visible_page": self.last_page()?,
        }))
    }
}

/// `Jikan\Parser\Magazine\MagazineListParser`.
pub struct MagazineListParser {
    doc: HtmlDoc,
}

impl MagazineListParser {
    pub fn new(doc: HtmlDoc) -> Self {
        MagazineListParser { doc }
    }

    /// `MagazineListParser::getMagazines()`.
    pub fn magazines(&self) -> Result<Vec<Value>, ParseError> {
        self.doc
            .css_nodes("a.genre-name-link")?
            .iter()
            .map(|node| MagazineListItemParser::new(node.clone()).model())
            .collect()
    }

    /// `MagazineListParser::getModel()`.
    pub fn model(&self) -> Result<Value, ParseError> {
        Ok(json!({ "magazines": self.magazines()? }))
    }
}

/// `Jikan\Parser\Magazine\MagazineListItemParser`.
pub struct MagazineListItemParser {
    node: crate::parser::helper::HtmlNode,
}

impl MagazineListItemParser {
    pub fn new(node: crate::parser::helper::HtmlNode) -> Self {
        MagazineListItemParser { node }
    }

    /// `MagazineListItemParser::getUrl()`.
    pub fn url(&self) -> String {
        format!(
            "{}{}",
            crate::request::BASE_URL,
            self.node.node_attr("href").unwrap_or_default()
        )
    }

    /// `MagazineListItemParser::getMalId()`.
    pub fn mal_id(&self) -> Option<i64> {
        mal_id_re()
            .captures(&self.url())
            .and_then(|caps| caps[1].parse().ok())
    }

    /// `MagazineListItemParser::getName()`.
    pub fn name(&self) -> String {
        name_re()
            .captures(&self.node.node_text())
            .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
            .unwrap_or_default()
    }

    /// `MagazineListItemParser::getCount()`.
    pub fn count(&self) -> i64 {
        let text = self.node.node_text();
        let Some(caps) = count_re().captures(&text) else {
            return 0;
        };
        if &caps[1] == "-" {
            return 0;
        }
        php_int_prefix(&caps[1].replace(',', ""))
    }

    /// `MagazineListItemParser::getModel()`.
    pub fn model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "mal_id": self.mal_id(),
            "name": self.name(),
            "url": self.url(),
            "count": self.count(),
        }))
    }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn php_int_prefix(input: &str) -> i64 {
    let trimmed = input.trim_start();
    let bytes = trimmed.as_bytes();
    let mut i = 0;
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
    let value: i64 = trimmed[start..i].parse().unwrap_or(0);
    if negative {
        -value
    } else {
        value
    }
}

fn page_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\?page=(\d+)$").expect("valid regex"))
}

fn mal_id_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(\d+)/.*$").expect("valid regex"))
}

fn name_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(.+)\s\(.*\)").expect("valid regex"))
}

fn count_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r".+\s\((.+)\)").expect("valid regex"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_item_regexes_match_php() {
        let doc = HtmlDoc::parse_str(
            r#"<div><a class="genre-name-link" href="/manga/magazine/1/Big_Comic_Original">Big Comic Original (1,234)</a>
            <a class="genre-name-link" href="/manga/magazine/2/Young_Jump">Young Jump (-)</a></div>"#,
        )
        .unwrap();
        let magazines = MagazineListParser::new(doc).magazines().unwrap();
        assert_eq!(magazines.len(), 2);
        assert_eq!(magazines[0]["mal_id"], 1);
        assert_eq!(magazines[0]["name"], "Big Comic Original");
        assert_eq!(magazines[0]["count"], 1234);
        assert_eq!(
            magazines[0]["url"],
            "https://myanimelist.net/manga/magazine/1/Big_Comic_Original"
        );
        assert_eq!(magazines[1]["count"], 0);
    }
}
