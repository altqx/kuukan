//! MAL URL helpers: `MalUrl` model, `MalUrlParser`, `MalUrlExtractor` and the
//! URL-id helpers from `Jikan\Helper\Parser`.
//!
//! JSON mappings mirror `App\Providers\SerializerFactory::convertMalUrl()`
//! (`{mal_id, type, name, url}`) and its v2 variant
//! (`{mal_id, type, title, name, url}`).

use regex::Regex;
use serde_json::{json, Value};
use std::sync::OnceLock;

use crate::error::ParseError;
use crate::parser::helper::HtmlNode;
use crate::parser::jstring::cleanse;

/// `Constants::BASE_URL`.
pub const BASE_URL: &str = "https://myanimelist.net";

/// `Constants::CDN_URL`.
pub const CDN_URL: &str = "https://cdn.myanimelist.net";

/// `MalUrlExtractor::TYPE_ANIME`.
pub const TYPE_ANIME: &str = "anime";

/// `MalUrlExtractor::TYPE_MANGA`.
pub const TYPE_MANGA: &str = "manga";

/// `Parser::idFromUrl()`.
pub fn id_from_url(url: &str) -> i64 {
    let replaced = id_from_url_re().replace(url, "$2");
    php_intval(&replaced)
}

/// `Parser::clubIdFromUrl()`.
pub fn club_id_from_url(url: &str) -> i64 {
    let replaced = club_id_re().replace(url, "$1");
    php_intval(&replaced)
}

/// `Parser::suffixIdFromUrl()`.
pub fn suffix_id_from_url(url: &str) -> i64 {
    let replaced = suffix_id_re().replace(url, "$1");
    php_intval(&replaced)
}

/// `Jikan\Model\Common\MalUrl`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MalUrl {
    name: String,
    url: String,
}

impl MalUrl {
    pub fn new(name: impl Into<String>, url: impl Into<String>) -> Self {
        MalUrl {
            name: name.into(),
            url: url.into(),
        }
    }

    /// `MalUrl::getMalId()`.
    pub fn mal_id(&self) -> i64 {
        MalUrlParser::parse_id(&self.url)
    }

    /// `MalUrl::getType()` (`preg_replace('#https://myanimelist.net/(\w+)/.*#', '$1', $url)`).
    pub fn r#type(&self) -> String {
        mal_url_type_re().replace(&self.url, "$1").to_string()
    }

    /// `MalUrl::getName()`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// `MalUrl::getTitle()` (same as `name`).
    pub fn title(&self) -> &str {
        &self.name
    }

    /// `MalUrl::getUrl()`.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// `SerializeNull`-style v4 payload: `{mal_id, type, name, url}`.
    pub fn to_json(&self) -> Value {
        json!({
            "mal_id": self.mal_id(),
            "type": self.r#type(),
            "name": self.title(),
            "url": self.url(),
        })
    }

    /// v2 payload: `{mal_id, type, title, name, url}`.
    pub fn to_json_v2(&self) -> Value {
        json!({
            "mal_id": self.mal_id(),
            "type": self.r#type(),
            "title": self.title(),
            "name": self.name(),
            "url": self.url(),
        })
    }
}

impl std::fmt::Display for MalUrl {
    /// `__toString()` returns the name.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

/// `Jikan\Parser\Common\MalUrlParser`.
pub struct MalUrlParser {
    node: HtmlNode,
}

impl MalUrlParser {
    pub fn new(node: HtmlNode) -> Self {
        MalUrlParser { node }
    }

    /// `MalUrlParser::parseId()`: first `/` + digits in the URL.
    pub fn parse_id(url: &str) -> i64 {
        match id_re().captures(url) {
            Some(caps) => caps[1].parse().unwrap_or(0),
            None => 0,
        }
    }

    /// `MalUrlParser::getModel()`.
    ///
    /// `$href = str_replace('https://myanimelist.net', '', $href)` and the
    /// name comes from `JString::cleanse($crawler->text())`.
    pub fn get_model(&self) -> Result<MalUrl, ParseError> {
        let href = self.node.node_attr("href").unwrap_or_default();
        let href = href.replace(BASE_URL, "");
        Ok(MalUrl::new(
            cleanse(&self.node.node_text()),
            format!("{BASE_URL}{href}"),
        ))
    }
}

/// `Jikan\Helper\MalUrlExtractor`.
///
/// Wraps an owned document/node (PHP clones the crawler) so the image-link
/// removal does not affect the caller's selection.
pub struct MalUrlExtractor {
    node: HtmlNode,
    kind: String,
    image_links: bool,
}

impl MalUrlExtractor {
    /// Construct from a whole document.
    pub fn new(
        doc: crate::parser::helper::HtmlDoc,
        kind: impl Into<String>,
        image_links: bool,
    ) -> Self {
        MalUrlExtractor {
            node: doc.root(),
            kind: kind.into(),
            image_links,
        }
    }

    /// Construct from a sub-selection node (PHP passes a filtered `Crawler`).
    pub fn from_node(node: HtmlNode, kind: impl Into<String>, image_links: bool) -> Self {
        MalUrlExtractor {
            node,
            kind: kind.into(),
            image_links,
        }
    }

    /// `MalUrlExtractor::getMalUrls()`.
    pub fn mal_urls(&self) -> Result<Vec<MalUrl>, ParseError> {
        if !self.image_links {
            // Remove the nearest element ancestor of every `<a><img ...></a>`
            // (PHP: `$c->ancestors()->first()` then `parentNode->removeChild`).
            for img in self.node.nodes("//a/img")? {
                if let Some(parent) = nearest_element_ancestor(&img) {
                    unsafe {
                        libxml::bindings::xmlUnlinkNode(parent.node().node_ptr());
                    }
                }
            }
        }

        let xpath = format!("//a[contains(@href, \"{}/{}\")]", BASE_URL, self.kind);
        let mut out = Vec::new();
        for anchor in self.node.nodes(&xpath)? {
            out.push(MalUrlParser::new(anchor).get_model()?);
        }
        Ok(out)
    }
}

fn nearest_element_ancestor(node: &HtmlNode) -> Option<HtmlNode> {
    let mut current = node.node().get_parent();
    while let Some(parent) = current {
        if parent.is_element_node() {
            return Some(HtmlNode::new(node.document().clone(), parent));
        }
        current = parent.get_parent();
    }
    None
}

/// PHP `(int) $string`: leading whitespace and sign, then digits; anything
/// non-numeric yields 0, overflow saturates.
fn php_intval(s: &str) -> i64 {
    let bytes = s.as_bytes();
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
    match s[start..i].parse::<i64>() {
        Ok(value) => {
            if negative {
                -value
            } else {
                value
            }
        }
        Err(_) => {
            if negative {
                i64::MIN
            } else {
                i64::MAX
            }
        }
    }
}

fn id_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"/(\d+)").expect("valid regex"))
}

fn id_from_url_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"https://myanimelist\.net(/\w+/)(\d+).*").expect("valid regex"))
}

fn club_id_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r".*\.php\?cid=(\d+)$").expect("valid regex"))
}

fn suffix_id_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"https://myanimelist\.net/.*/(\d+)").expect("valid regex"))
}

fn mal_url_type_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"https://myanimelist\.net/(\w+)/.*").expect("valid regex"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::helper::HtmlDoc;

    // Expected values captured from jikan-php (PHP 8.5).
    #[test]
    fn url_id_helpers() {
        let cases: &[(&str, i64, &str, i64, i64, i64)] = &[
            (
                "https://myanimelist.net/anime/1/Cowboy_Bebop",
                1,
                "anime",
                1,
                0,
                1,
            ),
            ("https://myanimelist.net/manga/2", 2, "manga", 2, 0, 2),
            (
                "https://myanimelist.net/anime.php?id=1",
                0,
                "https://myanimelist.net/anime.php?id=1",
                0,
                0,
                0,
            ),
            (
                "https://myanimelist.net/clubs.php?cid=12345",
                0,
                "https://myanimelist.net/clubs.php?cid=12345",
                0,
                12345,
                0,
            ),
            (
                "https://myanimelist.net/clubs.php?cid=12345&x=1",
                0,
                "https://myanimelist.net/clubs.php?cid=12345&x=1",
                0,
                0,
                0,
            ),
            (
                "https://myanimelist.net/forum/?topicid=999",
                0,
                "forum",
                0,
                0,
                0,
            ),
            (
                "https://myanimelist.net/anime/34599/episode/5",
                34599,
                "anime",
                34599,
                0,
                5,
            ),
            ("not a url", 0, "not a url", 0, 0, 0),
            ("", 0, "", 0, 0, 0),
        ];
        for (url, id, kind, id_from, club, suffix) in cases {
            let mal = MalUrl::new("name", *url);
            assert_eq!(mal.mal_id(), *id, "{url}");
            assert_eq!(&mal.r#type(), kind, "{url}");
            assert_eq!(id_from_url(url), *id_from, "{url}");
            assert_eq!(club_id_from_url(url), *club, "{url}");
            assert_eq!(suffix_id_from_url(url), *suffix, "{url}");
        }
    }

    #[test]
    fn parse_id_cases() {
        assert_eq!(MalUrlParser::parse_id("/anime/1"), 1);
        assert_eq!(MalUrlParser::parse_id("anime/1"), 1);
        assert_eq!(MalUrlParser::parse_id(""), 0);
        assert_eq!(MalUrlParser::parse_id("/1"), 1);
        assert_eq!(MalUrlParser::parse_id("abc"), 0);
        assert_eq!(MalUrlParser::parse_id("https://x/12"), 12);
    }

    #[test]
    fn json_mappings() {
        let mal = MalUrl::new(
            "Cowboy Bebop",
            "https://myanimelist.net/anime/1/Cowboy_Bebop",
        );
        assert_eq!(
            mal.to_json(),
            json!({
                "mal_id": 1,
                "type": "anime",
                "name": "Cowboy Bebop",
                "url": "https://myanimelist.net/anime/1/Cowboy_Bebop"
            })
        );
        assert_eq!(
            mal.to_json_v2(),
            json!({
                "mal_id": 1,
                "type": "anime",
                "title": "Cowboy Bebop",
                "name": "Cowboy Bebop",
                "url": "https://myanimelist.net/anime/1/Cowboy_Bebop"
            })
        );
        assert_eq!(mal.to_string(), "Cowboy Bebop");
    }

    #[test]
    fn extractor_removes_image_links() {
        let doc = HtmlDoc::parse_str(
            r#"
            <div>
              <a href="https://myanimelist.net/anime/1/Cowboy_Bebop"><img src="x.jpg"/></a>
              <a href="https://myanimelist.net/anime/2/Trigun">Trigun</a>
              <a href="https://myanimelist.net/manga/3/Berserk">Berserk</a>
            </div>
            "#,
        )
        .unwrap();

        // image links kept when requested
        let with_images = MalUrlExtractor::new(doc.clone(), TYPE_ANIME, true)
            .mal_urls()
            .unwrap();
        assert_eq!(with_images.len(), 2);

        // image links removed by default
        let extractor = MalUrlExtractor::new(doc, TYPE_ANIME, false);
        let urls = extractor.mal_urls().unwrap();
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].name(), "Trigun");
        assert_eq!(urls[0].mal_id(), 2);
        assert_eq!(urls[0].r#type(), "anime");
    }

    #[test]
    fn extractor_from_sub_node() {
        let doc = HtmlDoc::parse_str(
            r#"
            <div id="a"><a href="https://myanimelist.net/anime/1/A">A</a></div>
            <div id="b"><a href="https://myanimelist.net/anime/2/B">B</a></div>
            "#,
        )
        .unwrap();
        let node = doc.first("//div[@id='b']").unwrap().unwrap();
        let urls = MalUrlExtractor::from_node(node, TYPE_ANIME, false)
            .mal_urls()
            .unwrap();
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].name(), "B");
    }

    #[test]
    fn parser_reads_anchor() {
        let doc = HtmlDoc::parse_str(
            "<a href=\"https://myanimelist.net/anime/44/Trigun?q=1\">Trigun&nbsp;TV</a>",
        )
        .unwrap();
        let anchor = doc.first("//a").unwrap().unwrap();
        let mal = MalUrlParser::new(anchor).get_model().unwrap();
        assert_eq!(mal.name(), "Trigun TV");
        assert_eq!(mal.url(), "https://myanimelist.net/anime/44/Trigun?q=1");
        assert_eq!(mal.mal_id(), 44);
    }

    #[test]
    fn php_intval_behavior() {
        assert_eq!(php_intval("  -12abc"), -12);
        assert_eq!(php_intval("abc"), 0);
        assert_eq!(php_intval("+7"), 7);
        assert_eq!(php_intval("99999999999999999999"), i64::MAX);
    }
}
