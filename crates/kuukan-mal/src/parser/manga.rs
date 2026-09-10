//! Port of `Jikan\Parser\Manga\*` (jikan-php v4.0.12).
//!
//! The review-item parsing (`Jikan\Parser\Reviews\MangaReviewParser` and its
//! reviewer/reactions dependencies) is implemented locally because
//! `parser/reviews.rs` is owned by another crew and the exact shape is needed
//! by `get_manga_reviews()`.

use chrono::Datelike;
use regex::Regex;
use serde_json::{json, Map, Value};
use std::sync::OnceLock;

use crate::error::ParseError;
use crate::parser::character::{
    common_image_resource, user_image_resource, CharacterListItemParser,
};
use crate::parser::common::{alternative_titles, mal_url, url_parser};
use crate::parser::date::{format_atom, parse_date};
use crate::parser::helper::{parse_image_quality, HtmlDoc, HtmlNode};
use crate::parser::jstring::{cleanse, utf8_nbsp_trim};

// ---------------------------------------------------------------------------
// MangaParser
// ---------------------------------------------------------------------------

/// `Jikan\Parser\Manga\MangaParser`.
pub struct MangaParser {
    doc: HtmlDoc,
}

impl MangaParser {
    pub fn new(doc: HtmlDoc) -> Self {
        MangaParser { doc }
    }

    /// `MangaParser::getMangaURL()` (`og:url`).
    pub fn manga_url(&self) -> Result<Option<String>, ParseError> {
        self.doc.attr("//meta[@property='og:url']", "content")
    }

    /// `MangaParser::getMangaId()`.
    pub fn manga_id(&self) -> Result<i64, ParseError> {
        let url = self.manga_url()?.unwrap_or_default();
        Ok(match manga_id_re().captures(&url) {
            Some(caps) => caps[1].parse().unwrap_or(0),
            None => 0,
        })
    }

    /// `MangaParser::getMangaTitle()` (`og:title`).
    pub fn manga_title(&self) -> Result<Option<String>, ParseError> {
        self.doc.attr("//meta[@property='og:title']", "content")
    }

    /// `MangaParser::getMangaImageURL()` (`og:image`).
    pub fn manga_image_url(&self) -> Result<Option<String>, ParseError> {
        self.doc.attr("//meta[@property='og:image']", "content")
    }

    /// `MangaParser::getMangaSynopsis()` (uses the node's *inner HTML*).
    pub fn manga_synopsis(&self) -> Result<Option<String>, ParseError> {
        let Some(node) = self.doc.first("//span[@itemprop='description']")? else {
            return Ok(None);
        };
        let synopsis = cleanse(&node.node_html());
        if synopsis.starts_with("No synopsis information has been added to this title.") {
            return Ok(None);
        }
        Ok(Some(synopsis))
    }

    /// `MangaParser::getApproved()`.
    pub fn approved(&self) -> Result<bool, ParseError> {
        Ok(self
            .doc
            .count("//*[@id=\"addtolist\"]//span[contains(text(), \"pending approval\")]")?
            == 0)
    }

    /// `MangaParser::getMangaTitleEnglish()`.
    pub fn manga_title_english(&self) -> Result<Option<String>, ParseError> {
        self.labelled_value("English:")
    }

    /// `MangaParser::getMangaTitleSynonyms()`.
    pub fn manga_title_synonyms(&self) -> Result<Vec<String>, ParseError> {
        let Some(span) = self.labelled_span("Synonyms:")? else {
            return Ok(vec![]);
        };
        let Some(ancestor) = span.ancestors().into_iter().next() else {
            return Ok(vec![]);
        };
        let titles = ancestor.node_text().replace(&span.node_text(), "");
        Ok(titles.split(", ").map(cleanse).collect())
    }

    /// `MangaParser::getMangaTitleJapanese()`.
    pub fn manga_title_japanese(&self) -> Result<Option<String>, ParseError> {
        self.labelled_value("Japanese:")
    }

    /// `MangaParser::getTitles()`.
    ///
    /// `parser::common::alternative_titles()` returns only the section entries;
    /// the PHP prepends the `Default` title from `og:title`.
    pub fn titles(&self) -> Result<Vec<Value>, ParseError> {
        let mut titles = vec![json!({
            "type": "Default",
            "title": self.manga_title()?,
        })];
        titles.extend(alternative_titles(&self.doc)?);
        Ok(titles)
    }

    /// `MangaParser::getMangaType()`.
    pub fn manga_type(&self) -> Result<Option<String>, ParseError> {
        let Some(value) = self.labelled_value("Type:")? else {
            return Ok(None);
        };
        Ok(if value == "Unknown" {
            None
        } else {
            Some(value)
        })
    }

    /// `MangaParser::getMangaChapters()`.
    pub fn manga_chapters(&self) -> Result<Option<i64>, ParseError> {
        self.labelled_int("Chapters:")
    }

    /// `MangaParser::getMangaVolumes()`.
    pub fn manga_volumes(&self) -> Result<Option<i64>, ParseError> {
        self.labelled_int("Volumes:")
    }

    /// `MangaParser::getMangaStatus()`.
    pub fn manga_status(&self) -> Result<Option<String>, ParseError> {
        self.labelled_value("Status:")
    }

    /// `MangaParser::getMangaAuthors()`.
    pub fn manga_authors(&self) -> Result<Vec<Value>, ParseError> {
        self.doc
            .nodes("//span[text()=\"Authors:\"]/following-sibling::a")?
            .iter()
            .map(mal_url)
            .collect()
    }

    /// `MangaParser::getMangaSerialization()`.
    pub fn manga_serialization(&self) -> Result<Vec<Value>, ParseError> {
        self.doc
            .nodes("//span[text()=\"Serialization:\"]/following-sibling::a")?
            .iter()
            .map(mal_url)
            .collect()
    }

    /// `MangaParser::getGenres()`.
    pub fn genres(&self) -> Result<Vec<Value>, ParseError> {
        self.genre_links(&["Genres:", "Genre:"], true)
    }

    /// `MangaParser::getExplicitGenres()`.
    pub fn explicit_genres(&self) -> Result<Vec<Value>, ParseError> {
        self.genre_links(&["Explicit Genres:", "Explicit Genre:"], true)
    }

    /// `MangaParser::getDemographics()`.
    pub fn demographics(&self) -> Result<Vec<Value>, ParseError> {
        self.genre_links(&["Demographics:", "Demographic:"], false)
    }

    /// `MangaParser::getThemes()` (singular label is tried first).
    pub fn themes(&self) -> Result<Vec<Value>, ParseError> {
        self.genre_links(&["Theme:", "Themes:"], false)
    }

    /// `MangaParser::getScore()`.
    pub fn score(&self) -> Result<Option<f64>, ParseError> {
        let Some(node) = self.doc.first("//span[@itemprop=\"ratingValue\"]")? else {
            return Ok(None);
        };
        let score = cleanse(&node.node_text());
        if score == "N/A" {
            return Ok(None);
        }
        Ok(Some(php_float_prefix(&score)))
    }

    /// `MangaParser::getScoredBy()`.
    pub fn scored_by(&self) -> Result<Option<i64>, ParseError> {
        let Some(node) = self.doc.first("//span[@itemprop=\"ratingCount\"]")? else {
            return Ok(None);
        };
        let scored_by = cleanse(&node.node_text())
            .replace(',', "")
            .replace(" users", "")
            .replace(" user", "");
        if !is_numeric(&scored_by) {
            return Ok(None);
        }
        Ok(Some(php_int_prefix(&scored_by)))
    }

    /// `MangaParser::getMangaRank()`.
    pub fn rank(&self) -> Result<Option<i64>, ParseError> {
        let Some(span) = self.doc.first(
            "//div[@id=\"content\"]/table/tr/td[@class=\"borderClass\"]//span[text()=\"Ranked:\"]",
        )?
        else {
            return Ok(None);
        };
        let Some(ancestor) = span.ancestors().into_iter().next() else {
            return Ok(None);
        };
        ancestor.remove_child_nodes()?;
        let ranked = ancestor.node_text().replace('#', "");
        let ranked = ranked.trim();
        if ranked == "N/A" {
            return Ok(None);
        }
        Ok(Some(php_int_prefix(ranked)))
    }

    /// `MangaParser::getMangaPopularity()`.
    pub fn popularity(&self) -> Result<Option<i64>, ParseError> {
        let Some(span) = self.labelled_span("Popularity:")? else {
            return Ok(None);
        };
        let Some(ancestor) = span.ancestors().into_iter().next() else {
            return Ok(None);
        };
        let value = ancestor
            .node_text()
            .replace(&span.node_text(), "")
            .replace('#', "");
        Ok(Some(php_int_prefix(&cleanse(&value))))
    }

    /// `MangaParser::getMangaMembers()`.
    pub fn members(&self) -> Result<Option<i64>, ParseError> {
        self.comma_int("Members:")
    }

    /// `MangaParser::getMangaFavorites()`.
    pub fn favorites(&self) -> Result<Option<i64>, ParseError> {
        self.comma_int("Favorites:")
    }

    /// `MangaParser::getExternalLinks()`.
    pub fn external_links(&self) -> Result<Vec<Value>, ParseError> {
        self.doc
            .nodes(
                "//*[@id=\"content\"]/table//div[contains(@class, \"external_links\")]//a[contains(@class, \"link\") and not(contains(@class, \"js-more-links\"))]",
            )?
            .iter()
            .map(url_parser)
            .collect()
    }

    /// `MangaParser::getMangaRelated()`.
    ///
    /// MAL has divided relations into tiles and a table. Tiles append to one
    /// relation key, the table overwrites it (PHP semantics). With no
    /// relations at all PHP still returns `[]` (an empty array, not `{}`).
    pub fn related(&self) -> Result<Value, ParseError> {
        let mut related: Map<String, Value> = Map::new();

        let entries = self.doc.nodes(
            "//div[contains(@class, \"related-entries\")]/div[contains(@class, \"entries-tile\")]/div[contains(@class, \"entry\")]",
        )?;
        for entry in entries {
            let Some(relation_node) =
                entry.first("//div[@class=\"content\"]/div[@class=\"relation\"]")?
            else {
                continue;
            };
            let relation = cleanse(&relation_suffix_re().replace(&relation_node.node_text(), ""));
            let links = entry.nodes("//div[@class=\"content\"]/div[@class=\"title\"]/a")?;

            // Check for empty links #justMALThings
            if links.len() == 1 && links[0].node_text().is_empty() {
                related.insert(relation, json!([]));
                continue;
            }

            // Remove empty/bugged links from the document (the PHP crawler
            // still holds the detached nodes and reads the first one).
            for link in &links {
                if link.node_text().is_empty() {
                    unlink(link);
                }
            }

            let Some(first) = links.first() else {
                continue;
            };
            let value = mal_url(first)?;
            match related.get_mut(&relation) {
                Some(Value::Array(items)) => items.push(value),
                _ => {
                    related.insert(relation, json!([value]));
                }
            }
        }

        let rows = self
            .doc
            .nodes("//table[contains(@class, \"entries-table\")]/tr")?;
        for row in rows {
            let links = row.nodes("//td[2]//a")?;
            let relation = cleanse(&row.text("//td[1]")?.unwrap_or_default().replace(':', ""));

            if links.len() == 1 && links[0].node_text().is_empty() {
                related.insert(relation, json!([]));
                continue;
            }

            for link in &links {
                if link.node_text().is_empty() {
                    unlink(link);
                }
            }

            let mut items = Vec::with_capacity(links.len());
            for link in &links {
                items.push(mal_url(link)?);
            }
            related.insert(relation, Value::Array(items));
        }

        if related.is_empty() {
            return Ok(json!([]));
        }
        Ok(Value::Object(related))
    }

    /// `MangaParser::getMangaBackground()`.
    pub fn background(&self) -> Result<Option<String>, ParseError> {
        let Some(node) = self.doc.first("//span[@itemprop=\"description\"]/..")? else {
            return Ok(None);
        };
        node.remove_child_nodes()?;
        let background = node.node_text();
        if background.contains("No background information has been added to this title") {
            return Ok(None);
        }
        Ok(Some(cleanse(&background)))
    }

    /// `MangaParser::getPublished()`.
    pub fn published(&self) -> Result<Value, ParseError> {
        Ok(date_range_json(
            &self.manga_published_string()?.unwrap_or_default(),
        ))
    }

    /// `MangaParser::getMangaPublishedString()`.
    pub fn manga_published_string(&self) -> Result<Option<String>, ParseError> {
        self.labelled_value("Published:")
    }

    /// `MangaParser::getModel()`.
    pub fn model(&self) -> Result<Value, ParseError> {
        let status = self.manga_status()?;
        Ok(json!({
            "mal_id": self.manga_id()?,
            "url": self.manga_url()?,
            "title": self.manga_title()?,
            "title_english": self.manga_title_english()?,
            "title_synonyms": self.manga_title_synonyms()?,
            "approved": self.approved()?,
            "title_japanese": self.manga_title_japanese()?,
            "titles": self.titles()?,
            "status": status,
            "images": common_image_resource(self.manga_image_url()?.as_deref()),
            "type": self.manga_type()?,
            "volumes": self.manga_volumes()?,
            "chapters": self.manga_chapters()?,
            "publishing": status.as_deref() == Some("Publishing"),
            "published": self.published()?,
            "rank": self.rank()?,
            "score": self.score()?,
            "scored_by": self.scored_by()?,
            "popularity": self.popularity()?,
            "members": self.members()?,
            "favorites": self.favorites()?,
            "synopsis": self.manga_synopsis()?,
            "background": self.background()?,
            "related": self.related()?,
            "genres": self.genres()?,
            "explicit_genres": self.explicit_genres()?,
            "demographics": self.demographics()?,
            "themes": self.themes()?,
            "authors": self.manga_authors()?,
            "serializations": self.manga_serialization()?,
            "external_links": self.external_links()?,
        }))
    }

    // -- internals ----------------------------------------------------------

    /// `//div[@id="content"]/table/tr/td[@class="borderClass"]` + span label.
    fn labelled_span(&self, label: &str) -> Result<Option<HtmlNode>, ParseError> {
        self.doc.first(&format!(
            "//div[@id=\"content\"]/table/tr/td[@class=\"borderClass\"]//span[text()=\"{label}\"]"
        ))
    }

    /// `JString::cleanse(str_replace($span->text(), '', $span->ancestors()->text()))`.
    fn labelled_value(&self, label: &str) -> Result<Option<String>, ParseError> {
        let Some(span) = self.labelled_span(label)? else {
            return Ok(None);
        };
        let Some(ancestor) = span.ancestors().into_iter().next() else {
            return Ok(None);
        };
        Ok(Some(cleanse(
            &ancestor.node_text().replace(&span.node_text(), ""),
        )))
    }

    /// The chapters/volumes branch: `Unknown` -> null, else `(int)`.
    fn labelled_int(&self, label: &str) -> Result<Option<i64>, ParseError> {
        let Some(span) = self.labelled_span(label)? else {
            return Ok(None);
        };
        let Some(ancestor) = span.ancestors().into_iter().next() else {
            return Ok(None);
        };
        let value = ancestor.node_text().replace(&span.node_text(), "");
        if value.trim() == "Unknown" {
            return Ok(None);
        }
        Ok(Some(php_int_prefix(&value)))
    }

    /// `(int)JString::cleanse(str_replace([$label, ','], '', $ancestors->text()))`.
    fn comma_int(&self, label: &str) -> Result<Option<i64>, ParseError> {
        let Some(span) = self.labelled_span(label)? else {
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

    /// The `Genres:`/`Genre:`/`Demographics:`/`Themes:` branches.
    fn genre_links(&self, labels: &[&str], check_empty: bool) -> Result<Vec<Value>, ParseError> {
        for label in labels {
            let Some(span) = self.doc.first(&format!("//span[text()=\"{label}\"]"))? else {
                continue;
            };
            let Some(ancestor) = span.ancestors().into_iter().next() else {
                continue;
            };
            if check_empty
                && ancestor
                    .node_text()
                    .contains("No genres have been added yet")
            {
                continue;
            }
            return ancestor.nodes("//a")?.iter().map(mal_url).collect();
        }
        Ok(vec![])
    }
}

// ---------------------------------------------------------------------------
// CharactersParser (manga character list)
// ---------------------------------------------------------------------------

/// `Jikan\Parser\Manga\CharactersParser`.
pub struct CharactersParser {
    doc: HtmlDoc,
}

impl CharactersParser {
    pub fn new(doc: HtmlDoc) -> Self {
        CharactersParser { doc }
    }

    /// `CharactersParser::getCharacters()`.
    pub fn characters(&self) -> Result<Vec<Value>, ParseError> {
        let tables = self
            .doc
            .nodes("//div[contains(@class, \"manga-character-container\")]/table")?;
        let mut out = Vec::with_capacity(tables.len());
        for table in tables {
            let parser = CharacterListItemParser::new(table);
            out.push(json!({
                "character": parser.character_meta()?,
                "role": parser.role()?,
            }));
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// MoreInfoParser
// ---------------------------------------------------------------------------

/// `Jikan\Parser\Manga\MoreInfoParser`.
pub struct MoreInfoParser {
    doc: HtmlDoc,
}

impl MoreInfoParser {
    pub fn new(doc: HtmlDoc) -> Self {
        MoreInfoParser { doc }
    }

    /// `MoreInfoParser::getMoreInfo()`.
    pub fn more_info(&self) -> Result<Option<String>, ParseError> {
        let Some(node) = self.doc.first("//div[contains(@class, \"rightside\")]")? else {
            return Ok(None);
        };
        node.remove_child_nodes()?;
        let more_info = cleanse(&node.node_text());
        Ok(if more_info.is_empty() {
            None
        } else {
            Some(more_info)
        })
    }

    /// `MoreInfoParser::getModel()`.
    pub fn model(&self) -> Result<Value, ParseError> {
        Ok(json!({ "more_info": self.more_info()? }))
    }
}

// ---------------------------------------------------------------------------
// MangaStatsParser
// ---------------------------------------------------------------------------

/// `Jikan\Parser\Manga\MangaStatsParser`.
pub struct MangaStatsParser {
    doc: HtmlDoc,
}

impl MangaStatsParser {
    pub fn new(doc: HtmlDoc) -> Self {
        MangaStatsParser { doc }
    }

    fn statistic(&self, label: &str) -> Result<i64, ParseError> {
        let Some(span) = self.doc.first(&format!(
            "//div[@class=\"spaceit_pad\"]/span[contains(text(), '{label}')]"
        ))?
        else {
            return Ok(0);
        };
        let Some(ancestor) = span.ancestors().into_iter().next() else {
            return Ok(0);
        };
        Ok(sanitize_int(&ancestor.node_text()))
    }

    /// `MangaStatsParser::getReading()`.
    pub fn reading(&self) -> Result<i64, ParseError> {
        self.statistic("Reading:")
    }

    /// `MangaStatsParser::getCompleted()`.
    pub fn completed(&self) -> Result<i64, ParseError> {
        self.statistic("Completed:")
    }

    /// `MangaStatsParser::getOnHold()`.
    pub fn on_hold(&self) -> Result<i64, ParseError> {
        self.statistic("On-Hold:")
    }

    /// `MangaStatsParser::getDropped()`.
    pub fn dropped(&self) -> Result<i64, ParseError> {
        self.statistic("Dropped:")
    }

    /// `MangaStatsParser::getPlanToRead()`.
    pub fn plan_to_read(&self) -> Result<i64, ParseError> {
        self.statistic("Plan to Read:")
    }

    /// `MangaStatsParser::getTotal()`.
    pub fn total(&self) -> Result<i64, ParseError> {
        self.statistic("Total:")
    }

    /// `MangaStatsParser::getScores()`.
    pub fn scores(&self) -> Result<Value, ParseError> {
        // `//h2[text()="Score Stats"]/following-sibling::text()`
        if let Some(node) = self
            .doc
            .first("//h2[text()=\"Score Stats\"]/following-sibling::text()")?
        {
            if node
                .node_text()
                .contains("No scores have been recorded for this")
            {
                return Ok(json!([]));
            }
        }

        let rows = self
            .doc
            .nodes("//h2[text()=\"Score Stats\"]/following-sibling::table[1]/tr")?;
        let mut scores: Map<String, Value> = Map::new();
        for row in rows {
            let score = row
                .text("//td[1]")?
                .map(|text| php_int_prefix(&text))
                .unwrap_or(0);
            let votes = row
                .text("//td[2]/div/span/small")?
                .map(|text| sanitize_int(&text))
                .unwrap_or(0);
            let percentage = {
                let Some(span) = row.first("//td[2]/div/span")? else {
                    continue;
                };
                span.remove_child_nodes()?;
                let text = span.node_text().replace('%', "");
                let text = utf8_nbsp_trim(&text);
                php_float_prefix(&cleanse(&text))
            };
            scores.insert(
                score.to_string(),
                json!({
                    "score": score,
                    "votes": votes,
                    "percentage": percentage,
                }),
            );
        }

        for score in 1..=10 {
            scores
                .entry(score.to_string())
                .or_insert_with(|| json!({ "score": score, "votes": 0, "percentage": 0.0 }));
        }

        // ksort: string keys do not sort numerically, so rebuild in order.
        let mut sorted = Map::new();
        for score in 1..=10 {
            if let Some(value) = scores.get(&score.to_string()) {
                sorted.insert(score.to_string(), value.clone());
            }
        }
        Ok(Value::Object(sorted))
    }

    /// `MangaStatsParser::getModel()`.
    pub fn model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "reading": self.reading()?,
            "completed": self.completed()?,
            "on_hold": self.on_hold()?,
            "dropped": self.dropped()?,
            "plan_to_read": self.plan_to_read()?,
            "total": self.total()?,
            "scores": self.scores()?,
        }))
    }
}

// ---------------------------------------------------------------------------
// MangaReviewsParser (with a local port of Reviews\MangaReviewParser)
// ---------------------------------------------------------------------------

/// `Jikan\Parser\Manga\MangaReviewsParser`.
pub struct MangaReviewsParser {
    doc: HtmlDoc,
}

impl MangaReviewsParser {
    pub fn new(doc: HtmlDoc) -> Self {
        MangaReviewsParser { doc }
    }

    /// `MangaReviewsParser::getResults()`.
    ///
    /// Reuses the reviews crew's `parser::reviews::MangaReviewParser` getters
    /// but assembles `Jikan\Model\Manga\MangaReview` (no `entry` key, unlike
    /// the `FullMangaReview` model serialized by `get_model()`).
    pub fn results(&self) -> Result<Vec<Value>, ParseError> {
        let nodes = self.doc.nodes(
            "//div[contains(@class, \"rightside\")]//div[contains(@class, \"review-element\")]",
        )?;
        let mut results = Vec::with_capacity(nodes.len());
        for node in &nodes {
            let parser = crate::parser::reviews::MangaReviewParser::new(node);
            results.push(json!({
                "mal_id": parser.get_id()?,
                "url": parser.get_url()?,
                "type": parser.get_type()?.unwrap_or_else(|| "manga".to_string()),
                "reactions": parser.get_reactions()?,
                "date": parser.get_date()?.map(|date| format_atom(&date)),
                "review": parser.get_content()?,
                "score": parser.get_reviewer_score()?,
                "tags": parser.get_review_tag()?,
                "is_spoiler": parser.is_spoiler()?,
                "is_preliminary": parser.is_preliminary()?,
                "chapters_read": parser.get_chapters_read()?,
                "user": parser.get_reviewer()?,
            }));
        }
        Ok(results)
    }

    /// `MangaReviewsParser::hasNextPage()`.
    pub fn has_next_page(&self) -> Result<bool, ParseError> {
        Ok(self
            .doc
            .count("//*[@id=\"content\"]/table//a[contains(text(), \"More Reviews\")]")?
            > 0)
    }

    /// `MangaReviewsParser::getModel()`.
    pub fn model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "results": self.results()?,
            "has_next_page": self.has_next_page()?,
            "last_visible_page": 1,
        }))
    }
}

// ---------------------------------------------------------------------------
// MangaReviewScoresParser
// ---------------------------------------------------------------------------

/// `Jikan\Parser\Manga\MangaReviewScoresParser`.
///
/// `MangaReview::fromParser()` never calls `getMangaScores()`, so this is a
/// direct port kept for parity with the PHP namespace (`MangaReviewerParser`
/// lives in `parser/reviews.rs`, owned by the reviews crew).
pub struct MangaReviewScoresParser {
    node: HtmlNode,
}

impl MangaReviewScoresParser {
    pub fn new(node: HtmlNode) -> Self {
        MangaReviewScoresParser { node }
    }

    fn score(&self, row: usize) -> Result<i64, ParseError> {
        Ok(self
            .node
            .text(&format!("//table/tr[{row}]/td[2]"))?
            .map(|text| php_int_prefix(&text))
            .unwrap_or(0))
    }

    pub fn overall(&self) -> Result<i64, ParseError> {
        Ok(self
            .node
            .text("//table/tr[1]/td[2]/strong")?
            .map(|text| php_int_prefix(&text))
            .unwrap_or(0))
    }

    pub fn story(&self) -> Result<i64, ParseError> {
        self.score(2)
    }

    pub fn art(&self) -> Result<i64, ParseError> {
        self.score(3)
    }

    pub fn character(&self) -> Result<i64, ParseError> {
        self.score(4)
    }

    pub fn enjoyment(&self) -> Result<i64, ParseError> {
        self.score(5)
    }

    /// `MangaReviewScoresParser::getModel()`.
    pub fn model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "overall": self.overall()?,
            "story": self.story()?,
            "art": self.art()?,
            "character": self.character()?,
            "enjoyment": self.enjoyment()?,
        }))
    }
}

// ---------------------------------------------------------------------------
// MangaRecentlyUpdatedByUsersParser
// ---------------------------------------------------------------------------

/// `Jikan\Parser\Manga\MangaRecentlyUpdatedByUsersParser`.
pub struct MangaRecentlyUpdatedByUsersParser {
    doc: HtmlDoc,
}

impl MangaRecentlyUpdatedByUsersParser {
    pub fn new(doc: HtmlDoc) -> Self {
        MangaRecentlyUpdatedByUsersParser { doc }
    }

    /// `MangaRecentlyUpdatedByUsersParser::getResults()`.
    pub fn results(&self) -> Result<Vec<Value>, ParseError> {
        let Some(first) = self
            .doc
            .first("//table[@class=\"table-recently-updated\"]/tr[1]")?
        else {
            return Ok(vec![]);
        };
        first
            .next_all()
            .iter()
            .map(|row| MangaRecentlyUpdatedByUsersListParser::new(row.clone()).model())
            .collect()
    }

    /// `MangaRecentlyUpdatedByUsersParser::getHasNextPage()` (hardcoded false).
    pub fn has_next_page(&self) -> bool {
        false
    }

    /// `MangaRecentlyUpdatedByUsersParser::getLastPage()` (hardcoded 1).
    pub fn last_page(&self) -> i64 {
        1
    }

    /// `MangaRecentlyUpdatedByUsersParser::getModel()`.
    pub fn model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "results": self.results()?,
            "has_next_page": self.has_next_page(),
            "last_visible_page": self.last_page(),
        }))
    }
}

/// `Jikan\Parser\Manga\MangaRecentlyUpdatedByUsersListParser`.
/// Read/total pair parsed from a `/`-separated progress column; `None` on one
/// side means the profile showed `-`.
type ProgressPair = Option<(Option<i64>, Option<i64>)>;

struct MangaRecentlyUpdatedByUsersListParser {
    node: HtmlNode,
}

impl MangaRecentlyUpdatedByUsersListParser {
    fn new(node: HtmlNode) -> Self {
        MangaRecentlyUpdatedByUsersListParser { node }
    }

    /// `...::getUserMeta()` (username/url from the second `div` link, image
    /// from the inline `style`).
    fn user_meta(&self) -> Result<Value, ParseError> {
        let username = self.node.text("//td[1]/div[2]/a")?.unwrap_or_default();
        let url = self
            .node
            .attr("//td[1]/div[2]/a", "href")?
            .unwrap_or_default();
        let style = self
            .node
            .attr("//td[1]/div[1]/a", "style")?
            .unwrap_or_default()
            .replace("thumbs/", "")
            .replace("_thumb", "")
            .replace("background-image:url(", "")
            .replace(')', "");
        let image = parse_image_quality(&style);
        Ok(json!({
            "username": username,
            "url": url,
            "images": user_image_resource(Some(&image)),
        }))
    }

    /// `...::getScore()`.
    fn score(&self) -> Result<Option<i64>, ParseError> {
        let score = self.node.text("//td[2]")?.unwrap_or_default();
        if score == "-" {
            return Ok(None);
        }
        Ok(Some(php_int_prefix(&score)))
    }

    /// `...::getStatus()`.
    fn status(&self) -> Result<String, ParseError> {
        Ok(self.node.text("//td[3]")?.unwrap_or_default())
    }

    fn split_column(&self, xpath: &str) -> Result<ProgressPair, ParseError> {
        let Some(text) = self.node.text(xpath)? else {
            return Ok(None);
        };
        let node_text = text.trim().replace(' ', "");
        if node_text.is_empty() {
            return Ok(None);
        }
        let mut parts = node_text.split('/');
        let read = parts.next().unwrap_or("");
        let total = parts.next().unwrap_or("");
        Ok(Some((
            if read == "-" {
                None
            } else {
                Some(php_int_prefix(read))
            },
            if total == "-" {
                None
            } else {
                Some(php_int_prefix(total))
            },
        )))
    }

    /// `...::getVolumesRead()` / `getVolumesTotal()`.
    fn volumes(&self) -> Result<(Option<i64>, Option<i64>), ParseError> {
        Ok(self.split_column("//td[4]")?.unwrap_or((None, None)))
    }

    /// `...::getChaptersRead()` / `getChaptersTotal()`.
    fn chapters(&self) -> Result<(Option<i64>, Option<i64>), ParseError> {
        Ok(self.split_column("//td[5]")?.unwrap_or((None, None)))
    }

    /// `...::getDate()` — `new DateTimeImmutable($text, UTC)`.
    fn date(&self) -> Result<Option<String>, ParseError> {
        let Some(text) = self.node.text("//td[6]")? else {
            return Ok(None);
        };
        Ok(parse_date(&text).map(|date| format_atom(&date)))
    }

    fn model(&self) -> Result<Value, ParseError> {
        let (volumes_read, volumes_total) = self.volumes()?;
        let (chapters_read, chapters_total) = self.chapters()?;
        Ok(json!({
            "user": self.user_meta()?,
            "score": self.score()?,
            "status": self.status()?,
            "volumes_read": volumes_read,
            "volumes_total": volumes_total,
            "chapters_read": chapters_read,
            "chapters_total": chapters_total,
            "date": self.date()?,
        }))
    }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn unlink(node: &HtmlNode) {
    // PHP `$node->parentNode->removeChild($node)`; the node stays owned by the
    // document (detached, not freed) and remains readable through HtmlNode.
    unsafe {
        libxml::bindings::xmlUnlinkNode(node.node().node_ptr());
    }
}

fn sanitize_int(input: &str) -> i64 {
    let digits: String = input.chars().filter(char::is_ascii_digit).collect();
    digits.parse().unwrap_or(0)
}

/// PHP `(int)` on a string.
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

/// PHP `(float)` on a string (numeric prefix).
fn php_float_prefix(input: &str) -> f64 {
    let trimmed = input.trim_start();
    let bytes = trimmed.as_bytes();
    let mut i = 0;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    let mut seen_digit = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
        seen_digit = true;
    }
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
            seen_digit = true;
        }
    }
    if !seen_digit {
        return 0.0;
    }
    trimmed[..i].parse().unwrap_or(0.0)
}

/// PHP `is_numeric()` for the integer subset used by `getScoredBy()`.
fn is_numeric(input: &str) -> bool {
    let trimmed = input.trim();
    !trimmed.is_empty() && trimmed.bytes().all(|b| b.is_ascii_digit())
}

/// `DateRange` -> `{from, to, prop, string}` (JMS `convertDateRange`).
fn date_range_json(raw: &str) -> Value {
    let from = date_range_from(raw);
    let until = date_range_until(raw);
    json!({
        "from": from.as_ref().map(format_atom),
        "to": until.as_ref().map(format_atom),
        "prop": {
            "from": date_prop(from.as_ref()),
            "to": date_prop(until.as_ref()),
        },
        "string": raw,
    })
}

fn date_prop(date: Option<&chrono::DateTime<chrono::FixedOffset>>) -> Value {
    match date {
        Some(date) => json!({
            "day": date.day(),
            "month": date.month(),
            "year": date.year(),
        }),
        None => json!({ "day": null, "month": null, "year": null }),
    }
}

fn date_range_from(raw: &str) -> Option<chrono::DateTime<chrono::FixedOffset>> {
    if raw == "Not available" {
        return None;
    }
    let date = if raw.contains(" to ") {
        raw.split(" to ").next().unwrap_or(raw)
    } else {
        raw
    };
    parse_date(date)
}

fn date_range_until(raw: &str) -> Option<chrono::DateTime<chrono::FixedOffset>> {
    if !raw.contains(" to ") || raw.contains(" to ?") {
        return None;
    }
    let date = raw.split_once(" to ").map(|x| x.1).unwrap_or_default();
    parse_date(date)
}

fn manga_id_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"https?://myanimelist\.net/manga/(\d+)").expect("valid regex"))
}

fn relation_suffix_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\s\(.*\)").expect("valid regex"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_range_parsing_matches_php() {
        let value = date_range_json("Sep 21, 1999 to Nov 10, 2014");
        assert_eq!(value["from"], "1999-09-21T00:00:00+00:00");
        assert_eq!(value["to"], "2014-11-10T00:00:00+00:00");
        assert_eq!(value["prop"]["from"]["day"], 21);
        assert_eq!(value["prop"]["from"]["month"], 9);
        assert_eq!(value["prop"]["from"]["year"], 1999);

        let value = date_range_json("Aug 25, 1989 to ?");
        assert_eq!(value["from"], "1989-08-25T00:00:00+00:00");
        assert!(value["to"].is_null());
        assert!(value["prop"]["to"]["day"].is_null());
        assert_eq!(value["string"], "Aug 25, 1989 to ?");

        let value = date_range_json("Not available");
        assert!(value["from"].is_null());
        assert_eq!(value["string"], "Not available");

        let value = date_range_json("2011");
        assert_eq!(value["from"], "2011-01-01T00:00:00+00:00");
        assert_eq!(value["prop"]["from"]["month"], 1);
    }

    #[test]
    fn float_prefix() {
        assert_eq!(php_float_prefix("8.07"), 8.07);
        assert_eq!(php_float_prefix("0.5"), 0.5);
        assert_eq!(php_float_prefix("N/A"), 0.0);
        assert_eq!(php_float_prefix("-1.5x"), -1.5);
    }

    #[test]
    fn sanitize_and_int_prefix() {
        assert_eq!(sanitize_int("11,701"), 11701);
        assert_eq!(php_int_prefix(" 700 "), 700);
        assert_eq!(php_int_prefix("Unknown"), 0);
    }

    #[test]
    fn unknown_chapters_and_volumes_are_null() {
        let doc = HtmlDoc::parse_str(
            r#"<div id="content"><table><tr><td class="borderClass">
            <div class="spaceit_pad"><span class="dark_text">Chapters:</span>
            <span>Unknown</span></div>
            <div class="spaceit_pad"><span class="dark_text">Volumes:</span>
            <span>Unknown</span></div>
            </td></tr></table></div>"#,
        )
        .unwrap();
        let parser = MangaParser::new(doc);
        assert_eq!(parser.manga_chapters().unwrap(), None);
        assert_eq!(parser.manga_volumes().unwrap(), None);
    }

    #[test]
    fn no_scores_yields_empty_array() {
        let doc = HtmlDoc::parse_str(
            r#"<div><h2>Score Stats</h2>No scores have been recorded for this manga.<table><tr>
            <td>1</td><td><div><span>10.0%<small>10</small></span></div></td></tr></table></div>"#,
        )
        .unwrap();
        assert_eq!(MangaStatsParser::new(doc).scores().unwrap(), json!([]));
    }

    #[test]
    fn scores_fill_missing_entries_and_sort() {
        let doc = HtmlDoc::parse_str(
            r#"<div><h2>Score Stats</h2><table>
            <tr><td>10</td><td><div><span>28.1%<small>8,404</small></span></div></td></tr>
            <tr><td>9</td><td><div><span>26.2%<small>7,830</small></span></div></td></tr>
            </table></div>"#,
        )
        .unwrap();
        let scores = MangaStatsParser::new(doc).scores().unwrap();
        assert_eq!(scores["10"]["votes"], 8404);
        assert_eq!(scores["9"]["votes"], 7830);
        assert_eq!(scores["9"]["percentage"], 26.2);
        // Missing scores are filled with zeros; ksort puts 1 first.
        assert_eq!(
            scores["1"],
            json!({"score": 1, "votes": 0, "percentage": 0.0})
        );
        let keys: Vec<&String> = scores.as_object().unwrap().keys().collect();
        assert_eq!(keys.first().map(|k| k.as_str()), Some("1"));
        assert_eq!(keys.last().map(|k| k.as_str()), Some("10"));
    }

    #[test]
    fn related_tiles_append_and_table_overwrites() {
        let doc = HtmlDoc::parse_str(
            r#"
            <div class="related-entries">
              <div class="entries-tile">
                <div class="entry"><div class="content">
                  <div class="relation">Alternative version (Manga)</div>
                  <div class="title"><a href="https://myanimelist.net/manga/12/Naruto_Gaiden">Naruto Gaiden</a></div>
                </div></div>
                <div class="entry"><div class="content">
                  <div class="relation">Alternative version (Manga)</div>
                  <div class="title"><a href="https://myanimelist.net/manga/13/Other">Other</a></div>
                </div></div>
                <div class="entry"><div class="content">
                  <div class="relation">Side story (Novel)</div>
                  <div class="title"><a href="https://myanimelist.net/manga/14/Empty"></a></div>
                </div></div>
              </div>
            </div>
            <table class="entries-table">
              <tr><td>Adaptation:</td><td><a href="https://myanimelist.net/anime/20/Naruto">Naruto</a></td></tr>
            </table>
            "#,
        )
        .unwrap();
        let related = MangaParser::new(doc).related().unwrap();
        assert_eq!(related["Alternative version"][0]["mal_id"], 12);
        assert_eq!(related["Alternative version"][1]["mal_id"], 13);
        assert_eq!(related["Alternative version"][0]["name"], "Naruto Gaiden");
        assert_eq!(related["Side story"], json!([]));
        assert_eq!(related["Adaptation"][0]["mal_id"], 20);
        assert_eq!(related["Adaptation"][0]["type"], "anime");
    }
}
