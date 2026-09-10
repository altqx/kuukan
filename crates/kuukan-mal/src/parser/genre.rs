//! Port of `Jikan\Parser\Genre\*` (jikan-php v4.0.12).

use regex::Regex;
use serde_json::{json, Value};
use std::sync::OnceLock;

use crate::error::ParseError;
use crate::parser::common::{anime_card, manga_card};
use crate::parser::helper::HtmlDoc;
use crate::parser::jstring::cleanse;

/// `Jikan\Parser\Genre\AnimeGenreParser`.
pub struct AnimeGenreParser {
    doc: HtmlDoc,
}

impl AnimeGenreParser {
    pub fn new(doc: HtmlDoc) -> Self {
        AnimeGenreParser { doc }
    }

    /// `AnimeGenreParser::getResults()`.
    pub fn results(&self) -> Result<Vec<Value>, ParseError> {
        self.doc
            .css_nodes("div.seasonal-anime")?
            .iter()
            .map(anime_card)
            .collect()
    }

    /// `AnimeGenreParser::getUrl()`.
    pub fn url(&self) -> Result<String, ParseError> {
        Ok(self
            .doc
            .attr("//meta[@property=\"og:url\"]", "content")?
            .unwrap_or_default())
    }

    /// `AnimeGenreParser::getMalId()`.
    pub fn mal_id(&self) -> Result<i64, ParseError> {
        Ok(genre_id_from_url(&self.url()?))
    }

    /// `AnimeGenreParser::getName()` (`preg_replace('~(.*?)\sAnime~', '$1', ...)`).
    pub fn name(&self) -> Result<String, ParseError> {
        let name = match self.doc.first("//span[@class='di-ib mt4']")? {
            Some(span) => {
                span.remove_child_nodes()?;
                cleanse(&span.node_text())
            }
            None => String::new(),
        };
        Ok(anime_suffix_re().replace_all(&name, "$1").to_string())
    }

    /// `AnimeGenreParser::getDescription()`.
    pub fn description(&self) -> Result<String, ParseError> {
        let Some(node) = self.doc.first("//*[@id=\"content\"]/div[4]/p")? else {
            return Ok(String::new());
        };
        node.remove_child_nodes()?;
        Ok(cleanse(&node.node_html()))
    }

    /// `AnimeGenreParser::getCount()`.
    pub fn count(&self) -> Result<i64, ParseError> {
        let Some(node) = self.doc.first("//span[@class='di-ib mt4']/span")? else {
            return Ok(0);
        };
        Ok(digits_only(&node.node_text()))
    }

    /// `AnimeGenreParser::getLastPage()`.
    pub fn last_page(&self) -> Result<i64, ParseError> {
        last_page(&self.doc)
    }

    /// `AnimeGenreParser::getHasNextPage()`.
    pub fn has_next_page(&self) -> Result<bool, ParseError> {
        has_next_page(&self.doc)
    }

    /// `AnimeGenreParser::getModel()`.
    pub fn model(&self) -> Result<Value, ParseError> {
        // Same call order as `AnimeGenre::fromParser()`: `getCount()` must run
        // before `getName()`, which removes the inner `<span>` holding the
        // count from the document.
        let count = self.count()?;
        let results = self.results()?;
        let name = self.name()?;
        Ok(json!({
            "results": results,
            "mal_id": self.mal_id()?,
            "url": self.url()?,
            "name": name,
            "description": self.description()?,
            "count": count,
            "has_next_page": self.has_next_page()?,
            "last_visible_page": self.last_page()?,
        }))
    }
}

/// `Jikan\Parser\Genre\MangaGenreParser`.
pub struct MangaGenreParser {
    doc: HtmlDoc,
}

impl MangaGenreParser {
    pub fn new(doc: HtmlDoc) -> Self {
        MangaGenreParser { doc }
    }

    /// `MangaGenreParser::getResults()`.
    pub fn results(&self) -> Result<Vec<Value>, ParseError> {
        self.doc
            .css_nodes("div.seasonal-anime")?
            .iter()
            .map(manga_card)
            .collect()
    }

    /// `MangaGenreParser::getUrl()`.
    pub fn url(&self) -> Result<String, ParseError> {
        Ok(self
            .doc
            .attr("//meta[@property=\"og:url\"]", "content")?
            .unwrap_or_default())
    }

    /// `MangaGenreParser::getMalId()`.
    pub fn mal_id(&self) -> Result<i64, ParseError> {
        Ok(genre_id_from_url(&self.url()?))
    }

    /// `MangaGenreParser::getName()` (no "Manga" suffix stripping).
    pub fn name(&self) -> Result<String, ParseError> {
        match self.doc.first("//span[@class='di-ib mt4']")? {
            Some(span) => {
                span.remove_child_nodes()?;
                Ok(cleanse(&span.node_text()))
            }
            None => Ok(String::new()),
        }
    }

    /// `MangaGenreParser::getDescription()`.
    pub fn description(&self) -> Result<String, ParseError> {
        let Some(node) = self.doc.first("//*[@id=\"content\"]/div[4]/p")? else {
            return Ok(String::new());
        };
        node.remove_child_nodes()?;
        Ok(cleanse(&node.node_html()))
    }

    /// `MangaGenreParser::getCount()`.
    pub fn count(&self) -> Result<i64, ParseError> {
        let Some(node) = self.doc.first("//span[@class='di-ib mt4']/span")? else {
            return Ok(0);
        };
        Ok(digits_only(&node.node_text()))
    }

    /// `MangaGenreParser::getLastPage()`.
    pub fn last_page(&self) -> Result<i64, ParseError> {
        last_page(&self.doc)
    }

    /// `MangaGenreParser::getHasNextPage()`.
    pub fn has_next_page(&self) -> Result<bool, ParseError> {
        has_next_page(&self.doc)
    }

    /// `MangaGenreParser::getModel()`.
    pub fn model(&self) -> Result<Value, ParseError> {
        // Same call order as `MangaGenre::fromParser()`: count before name.
        let count = self.count()?;
        let results = self.results()?;
        let name = self.name()?;
        Ok(json!({
            "results": results,
            "mal_id": self.mal_id()?,
            "url": self.url()?,
            "name": name,
            "description": self.description()?,
            "count": count,
            "has_next_page": self.has_next_page()?,
            "last_visible_page": self.last_page()?,
        }))
    }
}

/// Shared genre-list parsing for anime and manga (`genre-link` columns).
fn genre_list(doc: &HtmlDoc, index: usize) -> Result<Vec<Value>, ParseError> {
    let xpath = format!(
        "//*[@class=\"genre-link\"][{index}]/div/div/a[@class=\"genre-name-link\"]"
    );
    doc.nodes(&xpath)?
        .iter()
        .map(|node| GenreListItemParser::new(node.clone()).model())
        .collect()
}

/// `Jikan\Parser\Genre\AnimeGenreListParser`.
pub struct AnimeGenreListParser {
    doc: HtmlDoc,
}

impl AnimeGenreListParser {
    pub fn new(doc: HtmlDoc) -> Self {
        AnimeGenreListParser { doc }
    }

    pub fn genres(&self) -> Result<Vec<Value>, ParseError> {
        genre_list(&self.doc, 1)
    }

    pub fn explicit_genres(&self) -> Result<Vec<Value>, ParseError> {
        genre_list(&self.doc, 2)
    }

    pub fn themes(&self) -> Result<Vec<Value>, ParseError> {
        genre_list(&self.doc, 3)
    }

    pub fn demographics(&self) -> Result<Vec<Value>, ParseError> {
        genre_list(&self.doc, 4)
    }

    /// `AnimeGenreListParser::getModel()`.
    pub fn model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "genres": self.genres()?,
            "explicit_genres": self.explicit_genres()?,
            "themes": self.themes()?,
            "demographics": self.demographics()?,
        }))
    }
}

/// `Jikan\Parser\Genre\MangaGenreListParser`.
pub struct MangaGenreListParser {
    doc: HtmlDoc,
}

impl MangaGenreListParser {
    pub fn new(doc: HtmlDoc) -> Self {
        MangaGenreListParser { doc }
    }

    pub fn genres(&self) -> Result<Vec<Value>, ParseError> {
        genre_list(&self.doc, 1)
    }

    pub fn explicit_genres(&self) -> Result<Vec<Value>, ParseError> {
        genre_list(&self.doc, 2)
    }

    pub fn themes(&self) -> Result<Vec<Value>, ParseError> {
        genre_list(&self.doc, 3)
    }

    pub fn demographics(&self) -> Result<Vec<Value>, ParseError> {
        genre_list(&self.doc, 4)
    }

    /// `MangaGenreListParser::getModel()`.
    pub fn model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "genres": self.genres()?,
            "explicit_genres": self.explicit_genres()?,
            "themes": self.themes()?,
            "demographics": self.demographics()?,
        }))
    }
}

/// `Jikan\Parser\Genre\AnimeGenreListItemParser` and its manga twin (the
/// parsers are byte-identical apart from the class).
pub struct GenreListItemParser {
    node: crate::parser::helper::HtmlNode,
}

impl GenreListItemParser {
    pub fn new(node: crate::parser::helper::HtmlNode) -> Self {
        GenreListItemParser { node }
    }

    /// `...::getUrl()`.
    pub fn url(&self) -> String {
        format!(
            "{}{}",
            crate::request::BASE_URL,
            self.node.node_attr("href").unwrap_or_default()
        )
    }

    /// `...::getMalId()`.
    pub fn mal_id(&self) -> Option<i64> {
        mal_id_re()
            .captures(&self.url())
            .and_then(|caps| caps[1].parse().ok())
    }

    /// `...::getName()`: the count is stripped of commas, not the name digits.
    pub fn name(&self) -> String {
        name_re()
            .captures(&self.node.node_text())
            .and_then(|caps| caps.get(1).map(|m| m.as_str().replace(',', "")))
            .unwrap_or_default()
    }

    /// `...::getCount()`.
    pub fn count(&self) -> i64 {
        count_re()
            .captures(&self.node.node_text())
            .and_then(|caps| caps.get(1).map(|m| m.as_str().replace(',', "")))
            .and_then(|value| value.parse().ok())
            .unwrap_or(0)
    }

    /// `...::getModel()`.
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

fn genre_id_from_url(url: &str) -> i64 {
    // (int) preg_replace('#https://myanimelist.net(/\w+/\w+/)(\d+).*#', '$2', $url)
    match genre_id_re().captures(url) {
        Some(caps) => caps[2].parse().unwrap_or(0),
        None => 0,
    }
}

fn digits_only(input: &str) -> i64 {
    let digits: String = input.chars().filter(char::is_ascii_digit).collect();
    digits.parse().unwrap_or(0)
}

fn last_page(doc: &HtmlDoc) -> Result<i64, ParseError> {
    let links = doc.nodes("//*[@id=\"content\"]/div[5]/div/a[contains(@class, \"link\")]")?;
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

fn has_next_page(doc: &HtmlDoc) -> Result<bool, ParseError> {
    let links = doc.nodes("//*[@id=\"content\"]/div[5]/div/a[contains(@class, \"link\")]")?;
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

fn genre_id_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"https://myanimelist\.net(/\w+/\w+/)(\d+).*").expect("valid regex")
    })
}

fn anime_suffix_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(.*?)\sAnime").expect("valid regex"))
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
    RE.get_or_init(|| Regex::new(r"(.+)\s\(.+\)").expect("valid regex"))
}

fn count_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r".+\s\((.+)\)").expect("valid regex"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anime_name_strips_the_suffix() {
        let doc = HtmlDoc::parse_str(
            r#"<div><span class="di-ib mt4">Action Anime<span>(4,369)</span></span></div>"#,
        )
        .unwrap();
        let parser = AnimeGenreParser::new(doc);
        // count() before name(): name() strips the count span from the DOM.
        assert_eq!(parser.count().unwrap(), 4369);
        assert_eq!(parser.name().unwrap(), "Action");
    }

    #[test]
    fn manga_name_keeps_the_suffix() {
        let doc = HtmlDoc::parse_str(
            r#"<div><span class="di-ib mt4">Action Manga<span>(8,332)</span></span></div>"#,
        )
        .unwrap();
        let parser = MangaGenreParser::new(doc);
        assert_eq!(parser.count().unwrap(), 8332);
        assert_eq!(parser.name().unwrap(), "Action Manga");
    }

    #[test]
    fn genre_id_from_url_uses_second_group() {
        assert_eq!(
            genre_id_from_url("https://myanimelist.net/anime/genre/1/Action"),
            1
        );
        assert_eq!(genre_id_from_url("https://example.com/anime/genre/9/X"), 0);
    }

    #[test]
    fn list_columns_are_parsed_by_index() {
        let doc = HtmlDoc::parse_str(
            r#"
            <div class="genre-link"><div><div><a class="genre-name-link" href="/anime/genre/1/Action">Action (4,369)</a></div></div></div>
            <div class="genre-link"><div><div><a class="genre-name-link" href="/anime/genre/9/Ecchi">Ecchi (2,199)</a></div></div></div>
            <div class="genre-link"><div><div><a class="genre-name-link" href="/anime/genre/10/Military">Military (300)</a></div></div></div>
            <div class="genre-link"><div><div><a class="genre-name-link" href="/anime/genre/27/Shounen">Shounen (3,700)</a></div></div></div>
            "#,
        )
        .unwrap();
        let parser = AnimeGenreListParser::new(doc);
        assert_eq!(parser.genres().unwrap()[0]["name"], "Action");
        assert_eq!(parser.genres().unwrap()[0]["count"], 4369);
        assert_eq!(parser.explicit_genres().unwrap()[0]["name"], "Ecchi");
        assert_eq!(parser.themes().unwrap()[0]["name"], "Military");
        assert_eq!(parser.demographics().unwrap()[0]["name"], "Shounen");
        assert_eq!(
            parser.genres().unwrap()[0]["url"],
            "https://myanimelist.net/anime/genre/1/Action"
        );
    }

    #[test]
    fn manga_list_columns_use_manga_urls() {
        let doc = HtmlDoc::parse_str(
            r#"
            <div class="genre-link"><div><div><a class="genre-name-link" href="/manga/genre/1/Action">Action (8,332)</a></div></div></div>
            <div class="genre-link"><div><div><a class="genre-name-link" href="/manga/genre/9/Ecchi">Ecchi (1,000)</a></div></div></div>
            "#,
        )
        .unwrap();
        let parser = MangaGenreListParser::new(doc);
        let genres = parser.genres().unwrap();
        assert_eq!(genres[0]["mal_id"], 1);
        assert_eq!(genres[0]["name"], "Action");
        assert_eq!(genres[0]["count"], 8332);
        assert_eq!(
            genres[0]["url"],
            "https://myanimelist.net/manga/genre/1/Action"
        );
        // Empty columns stay `[]`.
        assert_eq!(parser.themes().unwrap(), Vec::<Value>::new());
        let model = MangaGenreListParser::new(
            HtmlDoc::parse_str(
                r#"<div class="genre-link"><div><div><a class="genre-name-link" href="/manga/genre/1/Action">Action (8,332)</a></div></div></div>"#,
            )
            .unwrap(),
        )
        .model()
        .unwrap();
        assert_eq!(model["explicit_genres"], json!([]));
    }
}
