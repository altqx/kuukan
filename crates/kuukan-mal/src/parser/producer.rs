//! Port of `Jikan\Parser\Producer\*` (jikan-php v4.0.12).

use regex::Regex;
use serde_json::{json, Value};
use std::sync::OnceLock;

use crate::error::ParseError;
use crate::parser::character::wrap_image_resource;
use crate::parser::common::{anime_card, url_parser};
use crate::parser::date::{format_atom, parse_date_mdy_readable};
use crate::parser::helper::HtmlDoc;
use crate::parser::jstring::cleanse;
use crate::parser::mal_url::MalUrl;

/// `Jikan\Parser\Producer\ProducerParser`.
pub struct ProducerParser {
    doc: HtmlDoc,
}

impl ProducerParser {
    pub fn new(doc: HtmlDoc) -> Self {
        ProducerParser { doc }
    }

    /// `ProducerParser::getResults()`.
    pub fn results(&self) -> Result<Vec<Value>, ParseError> {
        self.doc
            .nodes(
                "//*[@id=\"content\"]/div[2]/div[contains(@class, \"js-categories-seasonal\")]/div[contains(@class, \"seasonal-anime\")]",
            )?
            .iter()
            .map(anime_card)
            .collect()
    }

    /// `ProducerParser::getUrl()`.
    pub fn url(&self) -> Result<Value, ParseError> {
        let name = self
            .doc
            .text("//*[@class=\"title-name\"]")?
            .map(|name| cleanse(&name))
            .unwrap_or_default();
        let href = self
            .doc
            .attr("//meta[@property=\"og:url\"]", "content")?
            .unwrap_or_default();
        Ok(MalUrl::new(name, href).to_json())
    }

    /// `ProducerParser::getLastPage()`.
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

    /// `ProducerParser::getHasNextPage()`.
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

    /// `ProducerParser::getTitles()`.
    pub fn titles(&self) -> Result<Vec<Value>, ParseError> {
        let mut titles = Vec::new();

        if let Some(node) = self
            .doc
            .first("//*[@id=\"contentWrapper\"]//h1[contains(@class, \"title-name\")]")?
        {
            titles.push(json!({ "type": "Default", "title": node.node_text() }));
        }

        if let Some(span) = self.doc.first("//span[text()=\"Japanese:\"]")? {
            if let Some(ancestor) = span.ancestors().into_iter().next() {
                titles.push(json!({
                    "type": "Japanese",
                    "title": cleanse(&ancestor.node_text().replace(&span.node_text(), "")),
                }));
            }
        }

        if let Some(span) = self.doc.first("//span[text()=\"Synonyms:\"]")? {
            if let Some(ancestor) = span.ancestors().into_iter().next() {
                let synonyms = ancestor.node_text().replace(&span.node_text(), "");
                for synonym in synonyms.split(", ") {
                    let synonym = cleanse(synonym);
                    // PHP `empty($titleSynonym)`: "" and "0" are skipped.
                    if synonym.is_empty() || synonym == "0" {
                        continue;
                    }
                    titles.push(json!({ "type": "Synonym", "title": synonym }));
                }
            }
        }

        Ok(titles)
    }

    /// `ProducerParser::getEstablished()`.
    pub fn established(&self) -> Result<Option<String>, ParseError> {
        let Some(span) = self.doc.first("//span[text()=\"Established:\"]")? else {
            return Ok(None);
        };
        let Some(ancestor) = span.ancestors().into_iter().next() else {
            return Ok(None);
        };
        let raw = cleanse(&ancestor.node_text().replace(&span.node_text(), ""));
        Ok(parse_date_mdy_readable(&raw).map(|date| format_atom(&date)))
    }

    /// `ProducerParser::getAbout()`.
    pub fn about(&self) -> Result<Option<String>, ParseError> {
        let Some(node) = self.doc.first(
            "//*[@id=\"content\"]/div[1]//div[contains(@class, \"spaceit_pad\")]/span[not(contains(@class, \"dark_text\"))]",
        )?
        else {
            return Ok(None);
        };
        Ok(Some(cleanse(&node.node_html())))
    }

    /// `ProducerParser::getFavorites()`.
    pub fn favorites(&self) -> Result<Option<i64>, ParseError> {
        let Some(span) = self.doc.first("//span[text()=\"Member Favorites:\"]")? else {
            return Ok(None);
        };
        let Some(ancestor) = span.ancestors().into_iter().next() else {
            return Ok(None);
        };
        let value = ancestor
            .node_text()
            .replace(&span.node_text(), "")
            .replace(',', "");
        Ok(Some(php_int_prefix(&cleanse(&value))))
    }

    /// `ProducerParser::getAnimeCount()`.
    pub fn anime_count(&self) -> Result<i64, ParseError> {
        let text = self
            .doc
            .text("//*[@id=\"content\"]/div[2]/div[contains(@class, \"navi-seasonal\")]/div/ul/li[1]")?
            .unwrap_or_default();
        let Some(caps) = count_re().captures(&text) else {
            return Ok(0);
        };
        Ok(php_int_prefix(&cleanse(&caps[1].replace(',', ""))))
    }

    /// `ProducerParser::getExternalLinks()`.
    ///
    /// PHP bug preserved: when the second group of links is empty the first
    /// group is discarded as well (`return []`).
    pub fn external_links(&self) -> Result<Vec<Value>, ParseError> {
        let available = self.doc.nodes(
            "//*[@id=\"content\"]/div[1]/div[contains(@class, \"user-profile-sns\")]/span//a",
        )?;
        if available.is_empty() {
            return Ok(vec![]);
        }
        let mut links = Vec::with_capacity(available.len());
        for node in &available {
            links.push(url_parser(node)?);
        }

        let resources = self
            .doc
            .nodes("//*[@id=\"content\"]/div[1]/div[contains(@class, \"pb16\")]/span//a")?;
        if resources.is_empty() {
            return Ok(vec![]);
        }
        for node in &resources {
            links.push(url_parser(node)?);
        }
        Ok(links)
    }

    /// `ProducerParser::getImages()`.
    pub fn images(&self) -> Result<Value, ParseError> {
        Ok(wrap_image_resource(
            self.doc
                .attr(
                    "//*[@id=\"content\"]/div[1]/div[contains(@class, \"logo\")]/img",
                    "data-src",
                )?
                .as_deref(),
        ))
    }

    /// `ProducerParser::getModel()`.
    pub fn model(&self) -> Result<Value, ParseError> {
        let url = self.url()?;
        let titles = self.titles()?;
        let name = titles
            .first()
            .and_then(|title| title.get("title"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        Ok(json!({
            "results": self.results()?,
            "mal_id": url["mal_id"],
            "url": url["url"],
            "images": self.images()?,
            "name": name,
            "titles": titles,
            "established": self.established()?,
            "favorites": self.favorites()?,
            "about": self.about()?,
            "external_links": self.external_links()?,
            "count": self.anime_count()?,
            "has_next_page": self.has_next_page()?,
            "last_visible_page": self.last_page()?,
        }))
    }
}

/// `Jikan\Parser\Producer\ProducerListParser`.
pub struct ProducerListParser {
    doc: HtmlDoc,
}

impl ProducerListParser {
    pub fn new(doc: HtmlDoc) -> Self {
        ProducerListParser { doc }
    }

    /// `ProducerListParser::getProducers()`.
    pub fn producers(&self) -> Result<Vec<Value>, ParseError> {
        self.doc
            .css_nodes("a.genre-name-link")?
            .iter()
            .map(|node| ProducerListItemParser::new(node.clone()).model())
            .collect()
    }

    /// `ProducerListParser::getModel()`.
    pub fn model(&self) -> Result<Value, ParseError> {
        Ok(json!({ "producers": self.producers()? }))
    }
}

/// `Jikan\Parser\Producer\ProducerListItemParser`.
pub struct ProducerListItemParser {
    node: crate::parser::helper::HtmlNode,
}

impl ProducerListItemParser {
    pub fn new(node: crate::parser::helper::HtmlNode) -> Self {
        ProducerListItemParser { node }
    }

    /// `ProducerListItemParser::getUrl()`.
    pub fn url(&self) -> String {
        format!(
            "{}{}",
            crate::request::BASE_URL,
            self.node.node_attr("href").unwrap_or_default()
        )
    }

    /// `ProducerListItemParser::getMalId()`.
    pub fn mal_id(&self) -> Option<i64> {
        mal_id_re()
            .captures(&self.url())
            .and_then(|caps| caps[1].parse().ok())
    }

    /// `ProducerListItemParser::getName()`.
    pub fn name(&self) -> String {
        name_re()
            .captures(&self.node.node_text())
            .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
            .unwrap_or_default()
    }

    /// `ProducerListItemParser::getCount()`.
    pub fn count(&self) -> i64 {
        let text = self.node.node_text();
        let Some(caps) = count_item_re().captures(&text) else {
            return 0;
        };
        if &caps[1] == "-" {
            return 0;
        }
        php_int_prefix(&caps[1].replace(',', ""))
    }

    /// `ProducerListItemParser::getModel()`.
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

fn count_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\((.*)\)").expect("valid regex"))
}

fn mal_id_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(\d+)/.*$").expect("valid regex"))
}

fn name_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(.+)\s\(.*\)").expect("valid regex"))
}

fn count_item_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r".+\s\((.+)\)").expect("valid regex"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_item_regexes_match_php() {
        let doc = HtmlDoc::parse_str(
            r#"<div id="content"><a class="genre-name-link" href="/anime/producer/1/Pierrot">Pierrot (306)</a>
            <a class="genre-name-link" href="/anime/producer/2/Bones">Bones (301)</a>
            <a class="genre-name-link" href="/anime/producer/3/Studio_Deen">Studio Deen (-)</a></div>"#,
        )
        .unwrap();
        let producers = ProducerListParser::new(doc).producers().unwrap();
        assert_eq!(producers.len(), 3);
        assert_eq!(producers[0]["mal_id"], 1);
        assert_eq!(producers[0]["name"], "Pierrot");
        assert_eq!(producers[0]["count"], 306);
        assert_eq!(
            producers[0]["url"],
            "https://myanimelist.net/anime/producer/1/Pierrot"
        );
        assert_eq!(producers[2]["count"], 0);
    }
}
