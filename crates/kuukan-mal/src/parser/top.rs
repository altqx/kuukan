//! Ports of `Jikan\Parser\Top\*`.
//!
//! All four top listings share [`TopListItemParser`], exactly like the PHP.

use regex::Regex;
use serde_json::{json, Value};
use std::sync::OnceLock;

use crate::error::ParseError;
use crate::parser::date::{format_atom, parse_date};
use crate::parser::helper::{parse_image_quality, HtmlDoc, HtmlNode};
use crate::parser::jstring::cleanse;
use crate::parser::mal_url::MalUrlParser;
use crate::parser::search::php_intval;

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

/// `CharacterImageResource::factory()`.
fn character_image_resource(image_url: Option<&str>) -> Value {
    match image_url {
        None => json!({
            "jpg": {"image_url": null},
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
    json!({ "jpg": {"image_url": image_url} })
}

/// `Jikan\Parser\Top\TopListItemParser`.
pub struct TopListItemParser<'a> {
    node: &'a HtmlNode,
}

impl<'a> TopListItemParser<'a> {
    pub fn new(node: &'a HtmlNode) -> Self {
        TopListItemParser { node }
    }

    /// `TopListItemParser::getMalUrl()`.
    pub fn get_mal_url(&self) -> Result<crate::parser::mal_url::MalUrl, ParseError> {
        // For Anime
        if let Some(node) = self
            .node
            .first("//td[contains(@class, \"title\")]/div/div/h3/a")?
        {
            return MalUrlParser::new(node).get_model();
        }
        // For Manga
        if let Some(node) = self
            .node
            .first("//td[contains(@class, \"title\")]/div/h3/a")?
        {
            return MalUrlParser::new(node).get_model();
        }
        // For Characters/People
        if let Some(node) = self
            .node
            .first("//td[contains(@class, \"people\")]/div/a")?
        {
            return MalUrlParser::new(node).get_model();
        }
        Err(ParseError::InvalidXPath(
            "Failed to parse MalUrl".to_string(),
        ))
    }

    /// `TopListItemParser::getImage()`.
    pub fn get_image(&self) -> Result<Option<String>, ParseError> {
        Ok(self
            .node
            .attr("//img[1]", "data-src")?
            .map(|src| parse_image_quality(&src)))
    }

    /// `TopListItemParser::getRank()`.
    pub fn get_rank(&self) -> Result<i64, ParseError> {
        Ok(php_intval(
            &self.node.text("//td[1]/span")?.unwrap_or_default(),
        ))
    }

    /// `TopListItemParser::getScore()`.
    pub fn get_score(&self) -> Result<f64, ParseError> {
        Ok(self
            .node
            .text("//td[3]/div/span")?
            .unwrap_or_default()
            .trim()
            .parse()
            .unwrap_or(0.0))
    }

    /// `TopListItemParser::getType()`.
    pub fn get_type(&self) -> Result<String, ParseError> {
        let text = self.get_text_array()?;
        let first = text.first().map(String::as_str).unwrap_or_default();
        Ok(type_re().replace(first, "$1").to_string())
    }

    /// `TopListItemParser::getTextArray()`.
    fn get_text_array(&self) -> Result<Vec<String>, ParseError> {
        let text = self.get_text()?;
        Ok(text
            .split('\n')
            .map(|part| {
                part.replace('\n', "")
                    .replace("<br>", "")
                    .trim()
                    .to_string()
            })
            .filter(|part| !part.is_empty())
            .collect())
    }

    /// `TopListItemParser::getText()`.
    fn get_text(&self) -> Result<String, ParseError> {
        let html = match self
            .node
            .first("//div[contains(@class, \"information\")]")?
        {
            Some(node) => node.node_html(),
            None => String::new(),
        };
        Ok(cleanse(&html))
    }

    /// `TopListItemParser::getEpisodes()`.
    pub fn get_episodes(&self) -> Result<Option<i64>, ParseError> {
        let text = self.get_text_array()?;
        let first = text.first().map(String::as_str).unwrap_or_default();
        let episodes = match episodes_re().captures(first) {
            Some(caps) => caps[1].parse().unwrap_or(0),
            None => php_intval(first),
        };
        Ok(if episodes == 0 { None } else { Some(episodes) })
    }

    /// `TopListItemParser::getVolumes()`.
    pub fn get_volumes(&self) -> Result<Option<i64>, ParseError> {
        let text = self.get_text_array()?;
        let first = text.first().map(String::as_str).unwrap_or_default();
        Ok(volumes_re()
            .captures(first)
            .and_then(|caps| caps[1].parse().ok()))
    }

    /// `TopListItemParser::getMembers()`.
    pub fn get_members(&self) -> Result<i64, ParseError> {
        let text = self.get_text_array()?;
        let part = text.get(2).map(String::as_str).unwrap_or_default();
        Ok(php_intval(&non_digits_re().replace_all(part, "")))
    }

    /// `TopListItemParser::getStartDate()`.
    pub fn get_start_date(&self) -> Result<Option<String>, ParseError> {
        let text = self.get_text_array()?;
        let part = text.get(1).map(String::as_str).unwrap_or_default();
        let date = cleanse(part.split('-').next().unwrap_or_default());
        Ok(if date.is_empty() { None } else { Some(date) })
    }

    /// `TopListItemParser::getEndDate()`.
    pub fn get_end_date(&self) -> Result<Option<String>, ParseError> {
        let text = self.get_text_array()?;
        let part = text.get(1).map(String::as_str).unwrap_or_default();
        // `explode('-', ...)[1] ?? '?'`
        let date = cleanse(part.split('-').nth(1).unwrap_or("?"));
        Ok(if date.is_empty() { None } else { Some(date) })
    }

    /// `TopListItemParser::getKanjiName()`.
    pub fn get_kanji_name(&self) -> Result<Option<String>, ParseError> {
        match self.node.first("//span[@class=\"fs12 fn-grey6\"][1]")? {
            Some(node) => {
                let text = node.node_text();
                Ok(Some(text.trim_matches(['(', ')']).to_string()))
            }
            None => Ok(None),
        }
    }

    /// `TopListItemParser::getAnimeography()`.
    pub fn get_animeography(&self) -> Result<Vec<Value>, ParseError> {
        let mut out = Vec::new();
        for node in self.node.nodes("//td[3]/div/a")? {
            out.push(MalUrlParser::new(node).get_model()?.to_json());
        }
        Ok(out)
    }

    /// `TopListItemParser::getMangaography()`.
    pub fn get_mangaography(&self) -> Result<Vec<Value>, ParseError> {
        let mut out = Vec::new();
        for node in self.node.nodes("//td[4]/div/a")? {
            out.push(MalUrlParser::new(node).get_model()?.to_json());
        }
        Ok(out)
    }

    /// `TopListItemParser::getFavorites()`.
    pub fn get_favorites(&self) -> Result<i64, ParseError> {
        let text = self.node.text("//td[5]")?.unwrap_or_default();
        Ok(php_intval(&non_digits_re().replace_all(&text, "")))
    }

    /// `TopListItemParser::getPeopleFavorites()`.
    pub fn get_people_favorites(&self) -> Result<i64, ParseError> {
        let text = self.node.text("//td[4]")?.unwrap_or_default();
        Ok(php_intval(&non_digits_re().replace_all(&text, "")))
    }

    /// `TopListItemParser::getBirthday()`.
    pub fn get_birthday(
        &self,
    ) -> Result<Option<chrono::DateTime<chrono::FixedOffset>>, ParseError> {
        let text = self.node.text("//td[3]")?.unwrap_or_default();
        Ok(parse_date(&text))
    }
}

/// `Jikan\Parser\Top\TopAnimeParser`.
pub struct TopAnimeParser<'a> {
    doc: &'a HtmlDoc,
}

impl<'a> TopAnimeParser<'a> {
    pub fn new(doc: &'a HtmlDoc) -> Self {
        TopAnimeParser { doc }
    }

    /// `TopAnime::fromParser()`.
    pub fn get_model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "results": self.get_results()?,
            "has_next_page": self.get_has_next_page()?,
            "last_visible_page": self.get_last_page()?,
        }))
    }

    /// `TopAnimeParser::getResults()`.
    pub fn get_results(&self) -> Result<Vec<Value>, ParseError> {
        let mut out = Vec::new();
        for node in self.doc.nodes("//tr[@class=\"ranking-list\"]")? {
            let item = TopListItemParser::new(&node);
            let mal_url = item.get_mal_url()?;
            let image = item.get_image()?;
            out.push(json!({
                "mal_id": mal_url.mal_id(),
                "rank": item.get_rank()?,
                "title": mal_url.title(),
                "url": mal_url.url(),
                "images": common_image_resource(image.as_deref()),
                "type": item.get_type()?,
                "episodes": item.get_episodes()?,
                "start_date": item.get_start_date()?,
                "end_date": item.get_end_date()?,
                "members": item.get_members()?,
                "score": item.get_score()?,
            }));
        }
        Ok(out)
    }

    /// `TopAnimeParser::getLastPage()`.
    pub fn get_last_page(&self) -> Result<i64, ParseError> {
        next_page_limit(
            self.doc,
            "//*[@id=\"content\"]/div[4]/h2/span[1]/a[contains(@class, \"next\")]",
        )
    }

    /// `TopAnimeParser::getHasNextPage()`.
    pub fn get_has_next_page(&self) -> Result<bool, ParseError> {
        Ok(self
            .doc
            .count("//*[@id=\"content\"]/div[4]/h2/span[1]/a[contains(@class, \"next\")]")?
            > 0)
    }
}

/// `Jikan\Parser\Top\TopMangaParser`.
pub struct TopMangaParser<'a> {
    doc: &'a HtmlDoc,
}

impl<'a> TopMangaParser<'a> {
    pub fn new(doc: &'a HtmlDoc) -> Self {
        TopMangaParser { doc }
    }

    /// `TopManga::fromParser()`.
    pub fn get_model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "results": self.get_results()?,
            "has_next_page": self.get_has_next_page()?,
            "last_visible_page": self.get_last_page()?,
        }))
    }

    /// `TopMangaParser::getResults()`.
    pub fn get_results(&self) -> Result<Vec<Value>, ParseError> {
        let mut out = Vec::new();
        for node in self.doc.nodes("//tr[@class=\"ranking-list\"]")? {
            let item = TopListItemParser::new(&node);
            let mal_url = item.get_mal_url()?;
            let image = item.get_image()?;
            out.push(json!({
                "mal_id": mal_url.mal_id(),
                "rank": item.get_rank()?,
                "title": mal_url.title(),
                "url": mal_url.url(),
                "images": common_image_resource(image.as_deref()),
                "type": item.get_type()?,
                "volumes": item.get_volumes()?,
                "start_date": item.get_start_date()?,
                "end_date": item.get_end_date()?,
                "members": item.get_members()?,
                "score": item.get_score()?,
            }));
        }
        Ok(out)
    }

    /// `TopMangaParser::getLastPage()`.
    pub fn get_last_page(&self) -> Result<i64, ParseError> {
        next_page_limit(
            self.doc,
            "//*[@id=\"content\"]/div[4]/h2/span[1]/a[contains(@class, \"next\")]",
        )
    }

    /// `TopMangaParser::getHasNextPage()`.
    pub fn get_has_next_page(&self) -> Result<bool, ParseError> {
        Ok(self
            .doc
            .count("//*[@id=\"content\"]/div[4]/h2/span[1]/a[contains(@class, \"next\")]")?
            > 0)
    }
}

/// `Jikan\Parser\Top\TopCharactersParser`.
pub struct TopCharactersParser<'a> {
    doc: &'a HtmlDoc,
}

impl<'a> TopCharactersParser<'a> {
    pub fn new(doc: &'a HtmlDoc) -> Self {
        TopCharactersParser { doc }
    }

    /// `TopCharacters::fromParser()`.
    pub fn get_model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "results": self.get_results()?,
            "has_next_page": self.get_has_next_page()?,
            "last_visible_page": self.get_last_page()?,
        }))
    }

    /// `TopCharactersParser::getResults()`.
    pub fn get_results(&self) -> Result<Vec<Value>, ParseError> {
        let mut out = Vec::new();
        for node in self.doc.nodes("//tr[@class=\"ranking-list\"]")? {
            let item = TopListItemParser::new(&node);
            let mal_url = item.get_mal_url()?;
            let image = item.get_image()?;
            out.push(json!({
                "mal_id": mal_url.mal_id(),
                "rank": item.get_rank()?,
                "title": mal_url.title(),
                "url": mal_url.url(),
                "images": character_image_resource(image.as_deref()),
                "name_kanji": item.get_kanji_name()?,
                "animeography": item.get_animeography()?,
                "mangaography": item.get_mangaography()?,
                "favorites": item.get_favorites()?,
            }));
        }
        Ok(out)
    }

    /// `TopCharactersParser::getLastPage()`.
    pub fn get_last_page(&self) -> Result<i64, ParseError> {
        next_page_limit(
            self.doc,
            "//*[@id=\"content\"]/h2/div/span/a[contains(@class, \"next\")]",
        )
    }

    /// `TopCharactersParser::getHasNextPage()`.
    pub fn get_has_next_page(&self) -> Result<bool, ParseError> {
        Ok(self
            .doc
            .count("//*[@id=\"content\"]/h2/div/span/a[contains(@class, \"next\")]")?
            > 0)
    }
}

/// `Jikan\Parser\Top\TopPeopleParser`.
pub struct TopPeopleParser<'a> {
    doc: &'a HtmlDoc,
}

impl<'a> TopPeopleParser<'a> {
    pub fn new(doc: &'a HtmlDoc) -> Self {
        TopPeopleParser { doc }
    }

    /// `TopPeople::fromParser()`.
    pub fn get_model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "results": self.get_results()?,
            "has_next_page": self.get_has_next_page()?,
            "last_visible_page": self.get_last_page()?,
        }))
    }

    /// `TopPeopleParser::getResults()`.
    pub fn get_results(&self) -> Result<Vec<Value>, ParseError> {
        let mut out = Vec::new();
        for node in self.doc.nodes("//tr[@class=\"ranking-list\"]")? {
            let item = TopListItemParser::new(&node);
            let mal_url = item.get_mal_url()?;
            let image = item.get_image()?;
            out.push(json!({
                "mal_id": mal_url.mal_id(),
                "rank": item.get_rank()?,
                "title": mal_url.title(),
                "url": mal_url.url(),
                "name_kanji": item.get_kanji_name()?,
                "favorites": item.get_people_favorites()?,
                "images": person_image_resource(image.as_deref()),
                "birthday": item.get_birthday()?.map(|date| format_atom(&date)),
            }));
        }
        Ok(out)
    }

    /// `TopPeopleParser::getLastPage()`.
    pub fn get_last_page(&self) -> Result<i64, ParseError> {
        next_page_limit(
            self.doc,
            "//*[@id=\"content\"]/h2/div/span/a[contains(@class, \"next\")]",
        )
    }

    /// `TopPeopleParser::getHasNextPage()`.
    pub fn get_has_next_page(&self) -> Result<bool, ParseError> {
        Ok(self
            .doc
            .count("//*[@id=\"content\"]/h2/div/span/a[contains(@class, \"next\")]")?
            > 0)
    }
}

/// `?limit=(\d+)` on the next link, divided by 50 and incremented.
fn next_page_limit(doc: &HtmlDoc, xpath: &str) -> Result<i64, ParseError> {
    let Some(href) = doc.attr(xpath, "href")? else {
        return Ok(1);
    };
    match limit_re().captures(&href) {
        Some(caps) => {
            let limit: i64 = caps[1].parse().unwrap_or(0);
            Ok(limit / 50 + 1)
        }
        None => Ok(1),
    }
}

fn type_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(.*)\s\(.*").expect("valid regex"))
}

fn episodes_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r".*\((\d+) eps\).*").expect("valid regex"))
}

fn volumes_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r".*\((\d+) vols\).*").expect("valid regex"))
}

fn non_digits_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\D").expect("valid regex"))
}

fn limit_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\?limit=(\d+)").expect("valid regex"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ranking_row() -> HtmlDoc {
        HtmlDoc::parse_str(
            r#"
            <table>
              <tr class="ranking-list">
                <td class="rank"><span>11</span></td>
                <td class="title">
                  <div>
                    <div><h3><a href="https://myanimelist.net/anime/42938/Fruits_Basket__The_Final">Fruits Basket: The Final</a></h3></div>
                    <div class="information">
                      TV (13 eps)<br>
                      Apr 2021 - Jun 2021<br>
                      376,544 members
                    </div>
                  </div>
                  <img class="lazyload" data-src="https://cdn.myanimelist.net/images/anime/1085/114792.jpg?s=x" />
                </td>
                <td class="score"><div><span>9.02</span></div></td>
              </tr>
            </table>
            "#,
        )
        .unwrap()
    }

    #[test]
    fn parses_anime_top_item() {
        let doc = ranking_row();
        let node = doc.first("//tr[@class=\"ranking-list\"]").unwrap().unwrap();
        let parser = TopAnimeParser::new(&doc);
        let results = parser.get_results().unwrap();
        let item = &results[0];
        assert_eq!(item["mal_id"], 42938);
        assert_eq!(item["rank"], 11);
        assert_eq!(item["title"], "Fruits Basket: The Final");
        assert_eq!(item["type"], "TV");
        assert_eq!(item["episodes"], 13);
        assert_eq!(item["start_date"], "Apr 2021");
        assert_eq!(item["end_date"], "Jun 2021");
        assert_eq!(item["members"], 376544);
        assert_eq!(item["score"], 9.02);
        let _ = node;
    }
}
