//! Ports of `Jikan\Parser\Search\*`.
//!
//! Search result pages are plain MAL tables; each parser mirrors the PHP
//! getters 1:1 (same XPath, same `JString::cleanse()` / `Parser::parseDate*()`
//! handling) and produces JMS-shaped `serde_json::Value` payloads.

use chrono::{DateTime, FixedOffset, Utc};
use regex::Regex;
use serde_json::{json, Value};
use std::sync::OnceLock;

use crate::error::ParseError;
use crate::parser::date::{format_atom, parse_date, parse_date_mdy, parse_date_time_pst};
use crate::parser::helper::{parse_image_quality, parse_image_thumb_to_hq, HtmlDoc, HtmlNode};
use crate::parser::jstring::{cleanse, utf8_nbsp_trim};
use crate::parser::mal_url::MalUrl;
use crate::parser::mal_url::{id_from_url, MalUrlParser, BASE_URL};

// ---------------------------------------------------------------------------
// Shared image resources (private to this module)
// ---------------------------------------------------------------------------

/// `CommonImageResource::factory()` (`{jpg, webp}` with 3 urls each).
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

/// `CharacterImageResource::factory()`.
fn character_image_resource(image_url: Option<&str>) -> Value {
    match image_url {
        None => json!({
            "jpg": {"image_url": null},
            // PHP `str_replace()` on null yields "" (the null check happens
            // after the replace), so the webp urls are empty strings.
            "webp": {"image_url": "", "small_image_url": ""},
        }),
        Some(url) => json!({
            "jpg": {"image_url": url},
            "webp": {
                "image_url": url.replace(".jpg", ".webp"),
                "small_image_url": url.replace(".jpg", "t.webp"),
            },
        }),
    }
}

/// `PersonImageResource::factory()`.
fn person_image_resource(image_url: Option<&str>) -> Value {
    json!({
        "jpg": {"image_url": image_url},
    })
}

/// `UserImageResource::factory()`.
fn user_image_resource(image_url: Option<&str>) -> Value {
    json!({
        "jpg": {"image_url": image_url},
        // `str_replace('.jpg', '.webp', null)` is "" in PHP.
        "webp": {"image_url": image_url.map(|url| url.replace(".jpg", ".webp")).unwrap_or_default()},
    })
}

// ---------------------------------------------------------------------------
// Anime search
// ---------------------------------------------------------------------------

/// `Jikan\Parser\Search\AnimeSearchParser`.
pub struct AnimeSearchParser<'a> {
    doc: &'a HtmlDoc,
}

impl<'a> AnimeSearchParser<'a> {
    pub fn new(doc: &'a HtmlDoc) -> Self {
        AnimeSearchParser { doc }
    }

    /// `AnimeSearch::fromParser()`:
    /// `{results, has_next_page, last_visible_page}`.
    pub fn get_model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "results": self.get_results()?,
            "has_next_page": self.get_has_next_page()?,
            "last_visible_page": self.get_last_page()?,
        }))
    }

    /// `AnimeSearchParser::getResults()`.
    pub fn get_results(&self) -> Result<Vec<Value>, ParseError> {
        let Some(header) = self
            .doc
            .first("//div[contains(@class, \"js-categories-seasonal\")]/table/tr[1]")?
        else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for node in header.next_all() {
            out.push(AnimeSearchListItemParser::new(&node).get_model()?);
        }
        Ok(out)
    }

    /// `AnimeSearchParser::getLastPage()`.
    pub fn get_last_page(&self) -> Result<i64, ParseError> {
        let Some(text) = self
            .doc
            .text("//div[contains(@class, \"normal_header\")]/div/div/span")?
        else {
            return Ok(1);
        };
        Ok(last_page_number(&text))
    }

    /// `AnimeSearchParser::getHasNextPage()`.
    pub fn get_has_next_page(&self) -> Result<bool, ParseError> {
        let Some(text) = self
            .doc
            .text("//div[contains(@class, \"normal_header\")]/div/div/span")?
        else {
            return Ok(false);
        };
        Ok(next_page_re().is_match(&text))
    }
}

/// `Jikan\Parser\Search\AnimeSearchListItemParser`.
pub struct AnimeSearchListItemParser<'a> {
    node: &'a HtmlNode,
}

impl<'a> AnimeSearchListItemParser<'a> {
    pub fn new(node: &'a HtmlNode) -> Self {
        AnimeSearchListItemParser { node }
    }

    /// `AnimeSearchListItem::fromParser()`.
    pub fn get_model(&self) -> Result<Value, ParseError> {
        let image_url = self.get_image_url()?;
        Ok(json!({
            "mal_id": id_from_url(&self.get_url()?),
            "url": self.get_url()?,
            "images": common_image_resource(Some(image_url.as_str())),
            "title": self.get_title()?,
            "airing": self.is_airing()?,
            "synopsis": self.get_synopsis()?,
            "type": self.get_type()?,
            "episodes": self.get_episodes()?,
            "score": self.get_score()?,
            "start_date": self.get_start_date()?.map(|date| format_atom(&date)),
            "end_date": self.get_end_date()?.map(|date| format_atom(&date)),
            "members": self.get_members()?,
            "rated": self.get_rated()?,
        }))
    }

    /// `AnimeSearchListItemParser::getUrl()`.
    pub fn get_url(&self) -> Result<String, ParseError> {
        Ok(self.node.attr("//td[2]//a", "href")?.unwrap_or_default())
    }

    /// `AnimeSearchListItemParser::getTitle()`.
    pub fn get_title(&self) -> Result<String, ParseError> {
        Ok(self.node.text("//td[2]//a/strong")?.unwrap_or_default())
    }

    /// `AnimeSearchListItemParser::getImageUrl()`.
    pub fn get_image_url(&self) -> Result<String, ParseError> {
        let src = self
            .node
            .attr("//td[1]/div/a/img", "data-src")?
            .unwrap_or_default();
        Ok(parse_image_quality(&src))
    }

    /// `AnimeSearchListItemParser::getSynopsis()`.
    pub fn get_synopsis(&self) -> Result<String, ParseError> {
        match self.node.first("//td[2]/div[@class=\"pt4\"]")? {
            Some(node) => {
                node.remove_child_nodes()?;
                Ok(cleanse(&node.node_text()))
            }
            None => Ok(String::new()),
        }
    }

    /// `AnimeSearchListItemParser::getType()`.
    pub fn get_type(&self) -> Result<String, ParseError> {
        Ok(cleanse(&self.node.text("//td[3]")?.unwrap_or_default()))
    }

    /// `AnimeSearchListItemParser::getEpisodes()`.
    pub fn get_episodes(&self) -> Result<i64, ParseError> {
        Ok(php_intval(&self.node.text("//td[4]")?.unwrap_or_default()))
    }

    /// `AnimeSearchListItemParser::getScore()`.
    pub fn get_score(&self) -> Result<f64, ParseError> {
        Ok(php_floatval(
            &self.node.text("//td[5]")?.unwrap_or_default(),
        ))
    }

    /// `AnimeSearchListItemParser::getMembers()`.
    pub fn get_members(&self) -> Result<i64, ParseError> {
        Ok(php_intval(
            &self
                .node
                .text("//td[8]")?
                .unwrap_or_default()
                .replace(',', ""),
        ))
    }

    /// `AnimeSearchListItemParser::getRated()`.
    pub fn get_rated(&self) -> Result<Option<String>, ParseError> {
        let rated = cleanse(&self.node.text("//td[9]")?.unwrap_or_default());
        if rated == "-" {
            return Ok(None);
        }
        Ok(Some(rated))
    }

    /// `AnimeSearchListItemParser::getStartDateString()`.
    pub fn get_start_date_string(&self) -> Result<Option<String>, ParseError> {
        let date = cleanse(&self.node.text("//td[6]")?.unwrap_or_default());
        if date == "-" {
            return Ok(None);
        }
        Ok(Some(date))
    }

    /// `AnimeSearchListItemParser::getStartDate()`.
    pub fn get_start_date(&self) -> Result<Option<DateTime<FixedOffset>>, ParseError> {
        match self.get_start_date_string()? {
            Some(date) => Ok(parse_date_mdy(Some(&date))),
            None => Ok(None),
        }
    }

    /// `AnimeSearchListItemParser::getEndDateString()`.
    pub fn get_end_date_string(&self) -> Result<Option<String>, ParseError> {
        let date = cleanse(&self.node.text("//td[7]")?.unwrap_or_default());
        if date == "-" {
            return Ok(None);
        }
        Ok(Some(date))
    }

    /// `AnimeSearchListItemParser::getEndDate()`.
    pub fn get_end_date(&self) -> Result<Option<DateTime<FixedOffset>>, ParseError> {
        match self.get_end_date_string()? {
            Some(date) => Ok(parse_date_mdy(Some(&date))),
            None => Ok(None),
        }
    }

    /// `AnimeSearchListItemParser::isAiring()`.
    pub fn is_airing(&self) -> Result<bool, ParseError> {
        // an entry with one episode can't be airing
        // its either finished airing or hasn't aired yet
        if self.get_episodes()? == 1 {
            return Ok(false);
        }
        // Start not yet known
        let Some(start_date) = self.get_start_date()? else {
            return Ok(false);
        };
        // Airing no end date
        let Some(end_date) = self.get_end_date()? else {
            return Ok(true);
        };
        let now = Utc::now().fixed_offset();
        // Not yet started
        if start_date > now {
            return Ok(false);
        }
        // Already ended
        if end_date < now {
            return Ok(false);
        }
        Ok(true)
    }
}

// ---------------------------------------------------------------------------
// Manga search
// ---------------------------------------------------------------------------

/// `Jikan\Parser\Search\MangaSearchParser`.
pub struct MangaSearchParser<'a> {
    doc: &'a HtmlDoc,
}

impl<'a> MangaSearchParser<'a> {
    pub fn new(doc: &'a HtmlDoc) -> Self {
        MangaSearchParser { doc }
    }

    /// `MangaSearch::fromParser()`.
    pub fn get_model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "results": self.get_results()?,
            "has_next_page": self.get_has_next_page()?,
            "last_visible_page": self.get_last_page()?,
        }))
    }

    /// `MangaSearchParser::getResults()`.
    pub fn get_results(&self) -> Result<Vec<Value>, ParseError> {
        let Some(header) = self
            .doc
            .first("//div[contains(@class, \"js-categories-seasonal\")]/table/tr[1]")?
        else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for node in header.next_all() {
            out.push(MangaSearchListItemParser::new(&node).get_model()?);
        }
        Ok(out)
    }

    /// `MangaSearchParser::getLastPage()`.
    pub fn get_last_page(&self) -> Result<i64, ParseError> {
        let Some(text) = self
            .doc
            .text("//div[contains(@class, \"normal_header\")]/div/div/span")?
        else {
            return Ok(1);
        };
        Ok(last_page_number(&text))
    }

    /// `MangaSearchParser::getHasNextPage()`.
    pub fn get_has_next_page(&self) -> Result<bool, ParseError> {
        let Some(text) = self
            .doc
            .text("//div[contains(@class, \"normal_header\")]/div/div/span")?
        else {
            return Ok(false);
        };
        Ok(next_page_re().is_match(&text))
    }
}

/// `Jikan\Parser\Search\MangaSearchListItemParser`.
pub struct MangaSearchListItemParser<'a> {
    node: &'a HtmlNode,
}

impl<'a> MangaSearchListItemParser<'a> {
    pub fn new(node: &'a HtmlNode) -> Self {
        MangaSearchListItemParser { node }
    }

    /// `MangaSearchListItem::fromParser()`.
    pub fn get_model(&self) -> Result<Value, ParseError> {
        let image_url = self.get_image_url()?;
        Ok(json!({
            "mal_id": id_from_url(&self.get_url()?),
            "url": self.get_url()?,
            "images": common_image_resource(Some(image_url.as_str())),
            "title": self.get_title()?,
            "publishing": self.is_publishing()?,
            "synopsis": self.get_synopsis()?,
            "type": self.get_type()?,
            "chapters": self.get_chapters()?,
            "volumes": self.get_volumes()?,
            "score": self.get_score()?,
            "start_date": self.get_start_date()?.map(|date| format_atom(&date)),
            "end_date": self.get_end_date()?.map(|date| format_atom(&date)),
            "members": self.get_members()?,
        }))
    }

    /// `MangaSearchListItemParser::getUrl()`.
    pub fn get_url(&self) -> Result<String, ParseError> {
        Ok(self.node.attr("//td[2]/a", "href")?.unwrap_or_default())
    }

    /// `MangaSearchListItemParser::getTitle()`.
    pub fn get_title(&self) -> Result<String, ParseError> {
        Ok(self.node.text("//td[2]/a/strong")?.unwrap_or_default())
    }

    /// `MangaSearchListItemParser::getImageUrl()`.
    pub fn get_image_url(&self) -> Result<String, ParseError> {
        let src = self
            .node
            .attr("//td[1]/div/a/img", "data-src")?
            .unwrap_or_default();
        Ok(parse_image_quality(&src))
    }

    /// `MangaSearchListItemParser::getSynopsis()`.
    pub fn get_synopsis(&self) -> Result<String, ParseError> {
        match self.node.first("//td[2]/div[@class=\"pt4\"]")? {
            Some(node) => {
                node.remove_child_nodes()?;
                Ok(cleanse(&node.node_text()))
            }
            None => Ok(String::new()),
        }
    }

    /// `MangaSearchListItemParser::getType()`.
    pub fn get_type(&self) -> Result<String, ParseError> {
        Ok(cleanse(&self.node.text("//td[3]")?.unwrap_or_default()))
    }

    /// `MangaSearchListItemParser::getVolumes()`.
    pub fn get_volumes(&self) -> Result<i64, ParseError> {
        Ok(php_intval(&self.node.text("//td[4]")?.unwrap_or_default()))
    }

    /// `MangaSearchListItemParser::getChapters()`.
    pub fn get_chapters(&self) -> Result<i64, ParseError> {
        Ok(php_intval(&self.node.text("//td[5]")?.unwrap_or_default()))
    }

    /// `MangaSearchListItemParser::getScore()`.
    pub fn get_score(&self) -> Result<f64, ParseError> {
        Ok(php_floatval(
            &self.node.text("//td[6]")?.unwrap_or_default(),
        ))
    }

    /// `MangaSearchListItemParser::getStartDateString()`.
    pub fn get_start_date_string(&self) -> Result<Option<String>, ParseError> {
        let date = cleanse(&self.node.text("//td[7]")?.unwrap_or_default());
        if date == "-" {
            return Ok(None);
        }
        Ok(Some(date))
    }

    /// `MangaSearchListItemParser::getStartDate()`.
    pub fn get_start_date(&self) -> Result<Option<DateTime<FixedOffset>>, ParseError> {
        match self.get_start_date_string()? {
            Some(date) => Ok(parse_date_mdy(Some(&date))),
            None => Ok(None),
        }
    }

    /// `MangaSearchListItemParser::getEndDateString()`.
    pub fn get_end_date_string(&self) -> Result<Option<String>, ParseError> {
        let date = cleanse(&self.node.text("//td[8]")?.unwrap_or_default());
        if date == "-" {
            return Ok(None);
        }
        Ok(Some(date))
    }

    /// `MangaSearchListItemParser::getEndDate()`.
    pub fn get_end_date(&self) -> Result<Option<DateTime<FixedOffset>>, ParseError> {
        match self.get_end_date_string()? {
            Some(date) => Ok(parse_date_mdy(Some(&date))),
            None => Ok(None),
        }
    }

    /// `MangaSearchListItemParser::getMembers()`.
    pub fn get_members(&self) -> Result<i64, ParseError> {
        Ok(php_intval(
            &self
                .node
                .text("//td[9]")?
                .unwrap_or_default()
                .replace(',', ""),
        ))
    }

    /// `MangaSearchListItem::fromParser()` inline `publishing` computation.
    pub fn is_publishing(&self) -> Result<bool, ParseError> {
        let end_date = self.get_end_date()?;
        let start_date = self.get_start_date()?;
        match (end_date, start_date) {
            (None, Some(start_date)) => Ok(Utc::now().fixed_offset() > start_date),
            _ => Ok(false),
        }
    }
}

// ---------------------------------------------------------------------------
// Character search
// ---------------------------------------------------------------------------

/// `Jikan\Parser\Search\CharacterSearchParser`.
pub struct CharacterSearchParser<'a> {
    doc: &'a HtmlDoc,
}

impl<'a> CharacterSearchParser<'a> {
    pub fn new(doc: &'a HtmlDoc) -> Self {
        CharacterSearchParser { doc }
    }

    /// `CharacterSearch::fromParser()`.
    pub fn get_model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "results": self.get_results()?,
            "has_next_page": self.get_has_next_page()?,
            "last_visible_page": self.get_last_page()?,
        }))
    }

    /// `CharacterSearchParser::getResults()`.
    pub fn get_results(&self) -> Result<Vec<Value>, ParseError> {
        if self.doc.count(
            "//div[@id=\"content\"]/table/tr/td[1][contains(text(), \"There were some probrems\")]",
        )? > 0
        {
            return Ok(Vec::new());
        }

        let mut out = Vec::new();
        for node in self.doc.nodes("//div[@id=\"content\"]/table/tr")? {
            out.push(CharacterSearchListItemParser::new(&node).get_model()?);
        }
        Ok(out)
    }

    /// `CharacterSearchParser::getLastPage()`.
    pub fn get_last_page(&self) -> Result<i64, ParseError> {
        let Some(text) = self
            .doc
            .text("//div[@id=\"content\"]/div[@class=\"borderClass\"][1]/div/span")?
        else {
            return Ok(1);
        };
        Ok(last_page_number(&text))
    }

    /// `CharacterSearchParser::getHasNextPage()`.
    pub fn get_has_next_page(&self) -> Result<bool, ParseError> {
        let Some(text) = self
            .doc
            .text("//div[contains(@class, \"normal_header\")]/div/div/span")?
        else {
            return Ok(false);
        };
        Ok(next_page_re().is_match(&text))
    }
}

/// `Jikan\Parser\Search\CharacterSearchListItemParser`.
pub struct CharacterSearchListItemParser<'a> {
    node: &'a HtmlNode,
}

impl<'a> CharacterSearchListItemParser<'a> {
    pub fn new(node: &'a HtmlNode) -> Self {
        CharacterSearchListItemParser { node }
    }

    /// `CharacterSearchListItem::fromParser()`.
    pub fn get_model(&self) -> Result<Value, ParseError> {
        let image_url = self.get_image_url()?;
        Ok(json!({
            "mal_id": id_from_url(&self.get_url()?),
            "url": self.get_url()?,
            "images": character_image_resource(Some(image_url.as_str())),
            "name": self.get_name()?,
            "alternative_names": self.get_alternative_names()?,
            "anime": self.get_anime()?,
            "manga": self.get_manga()?,
        }))
    }

    /// `CharacterSearchListItemParser::getUrl()`.
    pub fn get_url(&self) -> Result<String, ParseError> {
        Ok(self.node.attr("//td[2]/a", "href")?.unwrap_or_default())
    }

    /// `CharacterSearchListItemParser::getName()`.
    pub fn get_name(&self) -> Result<String, ParseError> {
        Ok(self.node.text("//td[2]/a")?.unwrap_or_default())
    }

    /// `CharacterSearchListItemParser::getAlternativeNames()`.
    pub fn get_alternative_names(&self) -> Result<Vec<String>, ParseError> {
        let Some(names) = self.node.first("//td[2]/small")? else {
            return Ok(Vec::new());
        };
        let text = names.node_text().replace(['(', ')'], "");
        Ok(text.split(',').map(cleanse).collect())
    }

    /// `CharacterSearchListItemParser::getImageUrl()`.
    pub fn get_image_url(&self) -> Result<String, ParseError> {
        let src = self
            .node
            .attr("//td[1]/div/a/img", "data-src")?
            .unwrap_or_default();
        Ok(parse_image_quality(&src))
    }

    /// `CharacterSearchListItemParser::getAnime()`.
    pub fn get_anime(&self) -> Result<Vec<Value>, ParseError> {
        // `Parser::removeChildNodes($crawler)` mutates every matched anchor.
        let anchors = self.node.nodes("//td[3]/small/a")?;
        if anchors.is_empty() {
            return Ok(Vec::new());
        }
        for anchor in &anchors {
            anchor.remove_child_nodes()?;
        }
        let mut out = Vec::new();
        for anchor in anchors {
            out.push(mal_url_json(&MalUrlParser::new(anchor).get_model()?));
        }
        Ok(out)
    }

    /// `CharacterSearchListItemParser::getManga()`.
    pub fn get_manga(&self) -> Result<Vec<Value>, ParseError> {
        let anchors = self.node.nodes("//td[3]/small/div/a")?;
        if anchors.is_empty() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for anchor in anchors {
            out.push(mal_url_json(&MalUrlParser::new(anchor).get_model()?));
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// Person search
// ---------------------------------------------------------------------------

/// `Jikan\Parser\Search\PersonSearchParser`.
pub struct PersonSearchParser<'a> {
    doc: &'a HtmlDoc,
}

impl<'a> PersonSearchParser<'a> {
    pub fn new(doc: &'a HtmlDoc) -> Self {
        PersonSearchParser { doc }
    }

    /// `PersonSearch::fromParser()`.
    pub fn get_model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "results": self.get_results()?,
            "has_next_page": self.get_has_next_page()?,
            "last_visible_page": self.get_last_page()?,
        }))
    }

    /// `PersonSearchParser::getResults()`.
    pub fn get_results(&self) -> Result<Vec<Value>, ParseError> {
        // if the query is empty, MAL returns a ranking of "Most Favorited" people
        // since that's not the scope of this method, we return empty results
        // most favorited people are returned via the `TopPeople` API method
        if self
            .doc
            .count("//*[@id=\"content\"]/table[@class=\"people-favorites-ranking-table\"]")?
            > 0
        {
            return Ok(Vec::new());
        }

        if self
            .doc
            .count("//div[@id=\"contentWrapper\"]/div[1]/div[@class=\"h1 edit-info\"]")?
            > 0
        {
            return Ok(vec![person_search_item_from_person_page(&self.doc.root())?]);
        }

        let mut out = Vec::new();
        for node in self.doc.nodes("//div[@id=\"content\"]/table/tr")? {
            out.push(PersonSearchListItemParser::new(&node).get_model()?);
        }
        Ok(out)
    }

    /// `PersonSearchParser::getLastPage()`.
    pub fn get_last_page(&self) -> Result<i64, ParseError> {
        let Some(text) = self
            .doc
            .text("//div[contains(@class, \"normal_header\")]/div/div/span")?
        else {
            return Ok(1);
        };
        Ok(last_page_number(&text))
    }

    /// `PersonSearchParser::getHasNextPage()`.
    pub fn get_has_next_page(&self) -> Result<bool, ParseError> {
        let Some(text) = self
            .doc
            .text("//div[contains(@class, \"normal_header\")]/div/div/span")?
        else {
            return Ok(false);
        };
        Ok(next_page_re().is_match(&text))
    }
}

/// `PersonSearchListItem::fromPersonParser()` (single-result redirect page).
fn person_search_item_from_person_page(root: &HtmlNode) -> Result<Value, ParseError> {
    let url = root
        .attr("//meta[@property='og:url']", "content")?
        .unwrap_or_default();
    let name = cleanse(
        &root
            .attr("//meta[@property='og:title']", "content")?
            .unwrap_or_default(),
    );
    let image_url = root
        .attr("//meta[@property='og:image']", "content")?
        .unwrap_or_default();
    let alternative_names = person_alternate_names(root)?;
    Ok(json!({
        "mal_id": id_from_url(&url),
        "url": url,
        "images": person_image_resource(Some(&image_url)),
        "name": name,
        "alternative_names": alternative_names,
    }))
}

/// `PersonParser::getPersonAlternateNames()`.
fn person_alternate_names(root: &HtmlNode) -> Result<Vec<String>, ParseError> {
    let Some(node) = root.first(
        "//div[@id=\"content\"]/table/tr/td[@class=\"borderClass\"]/div/span[text()=\"Alternate names:\"]",
    )?
    else {
        return Ok(Vec::new());
    };
    let label = node.node_text();
    let parent_text = node
        .ancestors()
        .first()
        .map(|ancestor| ancestor.node_text())
        .unwrap_or_default();
    let names = parent_text.replace(&label, "");
    Ok(names.split(',').map(cleanse).collect())
}

/// `Jikan\Parser\Search\PersonSearchListItemParser`.
pub struct PersonSearchListItemParser<'a> {
    node: &'a HtmlNode,
}

impl<'a> PersonSearchListItemParser<'a> {
    pub fn new(node: &'a HtmlNode) -> Self {
        PersonSearchListItemParser { node }
    }

    /// `PersonSearchListItem::fromParser()`.
    pub fn get_model(&self) -> Result<Value, ParseError> {
        let image_url = self.get_image_url()?;
        Ok(json!({
            "mal_id": id_from_url(&self.get_url()?),
            "url": self.get_url()?,
            "images": person_image_resource(Some(image_url.as_str())),
            "name": self.get_name()?,
            "alternative_names": self.get_alternative_names()?,
        }))
    }

    /// `PersonSearchListItemParser::getUrl()`.
    pub fn get_url(&self) -> Result<String, ParseError> {
        let href = self.node.attr("//td[2]/a", "href")?.unwrap_or_default();
        Ok(format!("{BASE_URL}{href}"))
    }

    /// `PersonSearchListItemParser::getName()`.
    pub fn get_name(&self) -> Result<String, ParseError> {
        Ok(self.node.text("//td[2]/a")?.unwrap_or_default())
    }

    /// `PersonSearchListItemParser::getAlternativeNames()`.
    pub fn get_alternative_names(&self) -> Result<Vec<String>, ParseError> {
        let Some(names) = self.node.first("//td[2]/small")? else {
            return Ok(Vec::new());
        };
        let text = names.node_text().replace(['(', ')'], "");
        Ok(text.split(',').map(cleanse).collect())
    }

    /// `PersonSearchListItemParser::getImageUrl()`.
    pub fn get_image_url(&self) -> Result<String, ParseError> {
        let src = self
            .node
            .attr("//td[1]/div/a/img", "data-src")?
            .unwrap_or_default();
        Ok(parse_image_quality(&src))
    }
}

// ---------------------------------------------------------------------------
// User search
// ---------------------------------------------------------------------------

/// `Jikan\Parser\Search\UserSearchParser`.
pub struct UserSearchParser<'a> {
    doc: &'a HtmlDoc,
}

impl<'a> UserSearchParser<'a> {
    pub fn new(doc: &'a HtmlDoc) -> Self {
        UserSearchParser { doc }
    }

    /// `UserSearch::fromParser()`.
    pub fn get_model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "results": self.get_results()?,
            "has_next_page": self.get_has_next_page()?,
            "last_visible_page": self.get_last_page()?,
        }))
    }

    /// `UserSearchParser::getResults()`.
    pub fn get_results(&self) -> Result<Vec<Value>, ParseError> {
        // Check if it's the main page (`getRecentlyOnlineUsers`).
        let mut nodes = self
            .doc
            .nodes("//*[@id=\"content\"]/table/tr/td[1]/table/tr/td")?;

        // User search page.
        if nodes.is_empty() {
            nodes = self.doc.nodes("//*[@id=\"content\"]/table/tr/td")?;
        }

        let mut data = Vec::new();
        for node in &nodes {
            data.push(UserSearchListItemParser::new(node).get_model()?);
        }

        // If only a single result is found, the $data array will be empty
        // (redirect occurs here to the User Page).
        if data.is_empty() {
            data.push(user_search_item_from_profile_page(&self.doc.root())?);
        }

        Ok(data)
    }

    /// `UserSearchParser::getLastPage()`.
    pub fn get_last_page(&self) -> Result<i64, ParseError> {
        let Some(text) = self
            .doc
            .text("//div[@id=\"content\"]/div[@class=\"borderClass\"][1]/div/span")?
        else {
            return Ok(1);
        };
        Ok(last_page_number(&text))
    }

    /// `UserSearchParser::getHasNextPage()`.
    pub fn get_has_next_page(&self) -> Result<bool, ParseError> {
        let Some(text) = self
            .doc
            .text("//div[@id=\"content\"]/div[@class=\"borderClass\"][1]/div/span")?
        else {
            return Ok(false);
        };
        Ok(next_page_re().is_match(&text))
    }
}

/// `UserSearchListItem::fromUserSearchParser()` (single-result user page).
fn user_search_item_from_profile_page(root: &HtmlNode) -> Result<Value, ParseError> {
    let url = root
        .attr("//meta[@property=\"og:url\"]", "content")?
        .unwrap_or_default();
    let username = profile_username_re().replace(&url, "$1").to_string();
    let image_url = root
        .first("//div[contains(@class, \"user-image\")]/img")?
        .and_then(|img| img.node_attr("data-src"));
    let last_online = root
        .text("//span[contains(text(), 'Last Online')]/following-sibling::span")?
        .map(|text| parse_date_time_pst(&text))
        .unwrap_or(None);
    Ok(json!({
        "username": username,
        "url": url,
        "images": user_image_resource(image_url.as_deref()),
        "last_online": last_online.map(|date| format_atom(&date)),
    }))
}

/// `Jikan\Parser\Search\UserSearchListItemParser`.
pub struct UserSearchListItemParser<'a> {
    node: &'a HtmlNode,
}

impl<'a> UserSearchListItemParser<'a> {
    pub fn new(node: &'a HtmlNode) -> Self {
        UserSearchListItemParser { node }
    }

    /// `UserSearchListItem::fromParser()`.
    pub fn get_model(&self) -> Result<Value, ParseError> {
        let image_url = self.get_image_url()?;
        Ok(json!({
            "username": self.get_username()?,
            "url": self.get_url()?,
            "images": user_image_resource(Some(image_url.as_str())),
            "last_online": self.get_last_online()?.map(|date| format_atom(&date)),
        }))
    }

    /// `UserSearchListItemParser::getUsername()`.
    pub fn get_username(&self) -> Result<String, ParseError> {
        Ok(self.node.text("//div[1]/a")?.unwrap_or_default())
    }

    /// `UserSearchListItemParser::getUrl()`.
    pub fn get_url(&self) -> Result<String, ParseError> {
        let href = self.node.attr("//div[1]/a", "href")?.unwrap_or_default();
        Ok(format!("{BASE_URL}{href}"))
    }

    /// `UserSearchListItemParser::getImageUrl()`.
    pub fn get_image_url(&self) -> Result<String, ParseError> {
        let src = self
            .node
            .attr("//div[2]/a/img", "data-src")?
            .unwrap_or_default();
        Ok(parse_image_thumb_to_hq(&src))
    }

    /// `UserSearchListItemParser::getLastOnline()`.
    pub fn get_last_online(&self) -> Result<Option<DateTime<FixedOffset>>, ParseError> {
        let last_online = utf8_nbsp_trim(&self.node.text("//div[3]/small")?.unwrap_or_default());
        Ok(parse_date(&last_online))
    }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// `MalUrl` JMS payload.
pub(crate) fn mal_url_json(mal_url: &MalUrl) -> Value {
    mal_url.to_json()
}

/// `explode(' ', $text)`, `end()`, `str_replace(['[', ']'], '', $last)`.
fn last_page_number(text: &str) -> i64 {
    let last = text.split(' ').next_back().unwrap_or_default();
    let cleaned = last.replace(['[', ']'], "");
    php_intval(&cleaned)
}

/// PHP `(int)` on a numeric string.
pub(crate) fn php_intval(s: &str) -> i64 {
    let trimmed = s.trim_start();
    let (negative, digits) = match trimmed.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, trimmed.strip_prefix('+').unwrap_or(trimmed)),
    };
    let digits: String = digits.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return 0;
    }
    match digits.parse::<i64>() {
        Ok(value) => {
            if negative {
                -value
            } else {
                value
            }
        }
        // PHP saturates on overflow.
        Err(_) => {
            if negative {
                i64::MIN
            } else {
                i64::MAX
            }
        }
    }
}

/// PHP `(float)` on a numeric string (empty/non-numeric => 0.0).
pub(crate) fn php_floatval(s: &str) -> f64 {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return 0.0;
    }
    if let Ok(value) = trimmed.parse::<f64>() {
        return value;
    }
    // PHP scans a numeric prefix like `(float) "8.5 abc"`.
    match php_float_prefix_re().find(trimmed) {
        Some(found) => found.as_str().parse().unwrap_or(0.0),
        None => 0.0,
    }
}

fn php_float_prefix_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^[+-]?(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?").expect("valid regex")
    })
}

fn next_page_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\[\d+]\s(\d+)").expect("valid regex"))
}

fn profile_username_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r".*/(.*)$").expect("valid regex"))
}
