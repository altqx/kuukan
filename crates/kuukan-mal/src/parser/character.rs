//! Port of `Jikan\Parser\Character\*` (jikan-php v4.0.12).
//!
//! Besides the character endpoints this module hosts the small JMS-shaped
//! "meta" builders (`anime_meta`, `manga_meta`, `character_meta`,
//! `person_meta`, image resources) that the other families reuse until a
//! shared home exists in `parser::common`.

use serde_json::{json, Value};

use crate::error::ParseError;
use crate::parser::common::mal_url;
use crate::parser::helper::{parse_image_quality, HtmlDoc, HtmlNode};
use crate::parser::jstring::{cleanse, utf8_nbsp_trim};
use crate::parser::mal_url::id_from_url;

// ---------------------------------------------------------------------------
// JMS-shaped resource/meta builders
// ---------------------------------------------------------------------------

/// `CommonImageResource` (`{jpg:{image_url,small_image_url,large_image_url},webp:{...}}`).
///
/// The webp factory does `str_replace('.jpg', ..., $imageUrl)`, which turns a
/// `null` input into `""` (PHP 8 deprecation, not an error) — the null case
/// therefore only affects `jpg.image_url`.
pub fn common_image_resource(image_url: Option<&str>) -> Value {
    let jpg = match image_url {
        Some(url) => json!({
            "image_url": url,
            "small_image_url": url.replace(".jpg", "t.jpg"),
            "large_image_url": url.replace(".jpg", "l.jpg"),
        }),
        None => json!({
            "image_url": null,
            "small_image_url": null,
            "large_image_url": null,
        }),
    };
    // `str_replace('.jpg', '.webp', null)` returns "" instead of null.
    let webp_source = image_url.unwrap_or_default();
    json!({
        "jpg": jpg,
        "webp": {
            "image_url": webp_source.replace(".jpg", ".webp"),
            "small_image_url": webp_source.replace(".jpg", "t.webp"),
            "large_image_url": webp_source.replace(".jpg", "l.webp"),
        },
    })
}

/// `CharacterImageResource` (`{jpg:{image_url},webp:{image_url,small_image_url}}`).
pub fn character_image_resource(image_url: Option<&str>) -> Value {
    let (webp_url, webp_small) = match image_url {
        Some(url) => (url.replace(".jpg", ".webp"), url.replace(".jpg", "t.webp")),
        // `str_replace` with null -> "" (and the null guard never triggers).
        None => (String::new(), String::new()),
    };
    json!({
        "jpg": { "image_url": image_url },
        "webp": {
            "image_url": webp_url,
            "small_image_url": webp_small,
        },
    })
}

/// `PersonImageResource` (`{jpg:{image_url}}`).
pub fn person_image_resource(image_url: Option<&str>) -> Value {
    json!({ "jpg": { "image_url": image_url } })
}

/// `UserImageResource` (`{jpg:{image_url},webp:{image_url}}`).
pub fn user_image_resource(image_url: Option<&str>) -> Value {
    json!({
        "jpg": { "image_url": image_url },
        "webp": {
            "image_url": image_url.unwrap_or_default().replace(".jpg", ".webp"),
        },
    })
}

/// `WrapImageResource` (`{jpg:{image_url}}`) used by producer / club logos.
pub fn wrap_image_resource(image_url: Option<&str>) -> Value {
    json!({ "jpg": { "image_url": image_url } })
}

/// `Jikan\Model\Common\AnimeMeta`.
pub fn anime_meta(title: &str, url: &str, image_url: Option<&str>) -> Value {
    let image = image_url
        .map(parse_image_quality)
        .map(Value::String)
        .unwrap_or(Value::Null);
    json!({
        "mal_id": id_from_url(url),
        "url": url,
        "images": common_image_resource(image.as_str()),
        "title": title,
    })
}

/// `Jikan\Model\Common\MangaMeta`.
pub fn manga_meta(title: &str, url: &str, image_url: Option<&str>) -> Value {
    let image = image_url
        .map(parse_image_quality)
        .map(Value::String)
        .unwrap_or(Value::Null);
    json!({
        "mal_id": id_from_url(url),
        "url": url,
        "images": common_image_resource(image.as_str()),
        "title": title,
    })
}

/// `Jikan\Model\Common\PersonMeta`.
pub fn person_meta(name: &str, url: &str, image_url: Option<&str>) -> Value {
    let image = image_url
        .map(parse_image_quality)
        .map(Value::String)
        .unwrap_or(Value::Null);
    json!({
        "mal_id": id_from_url(url),
        "url": url,
        "images": person_image_resource(image.as_str()),
        "name": name,
    })
}

/// `Jikan\Model\Common\CharacterMeta`.
pub fn character_meta(name: &str, url: &str, image_url: Option<&str>) -> Value {
    json!({
        "mal_id": id_from_url(url),
        "url": url,
        "images": character_image_resource(image_url),
        "name": name,
    })
}

// ---------------------------------------------------------------------------
// CharacterParser
// ---------------------------------------------------------------------------

/// `Jikan\Parser\Character\CharacterParser`.
pub struct CharacterParser {
    doc: HtmlDoc,
}

impl CharacterParser {
    pub fn new(doc: HtmlDoc) -> Self {
        CharacterParser { doc }
    }

    /// `CharacterParser::getCharacterUrl()` (`og:url`).
    pub fn character_url(&self) -> Result<Option<String>, ParseError> {
        self.doc.attr("//meta[@property=\"og:url\"]", "content")
    }

    /// `CharacterParser::getMalId()`.
    pub fn mal_id(&self) -> Result<i64, ParseError> {
        Ok(id_from_url(&self.character_url()?.unwrap_or_default()))
    }

    /// `CharacterParser::getName()` (`og:title`).
    pub fn name(&self) -> Result<Option<String>, ParseError> {
        self.doc.attr("//meta[@property=\"og:title\"]", "content")
    }

    /// `CharacterParser::getNameKanji()`.
    pub fn name_kanji(&self) -> Result<Option<String>, ParseError> {
        let Some(node) = self
            .doc
            .first("//h2[contains(@class, \"normal_header\")]/span/small")?
        else {
            return Ok(None);
        };
        Ok(Some(node.node_text().replace(['(', ')'], "")))
    }

    /// `CharacterParser::getNameNicknames()`.
    pub fn nicknames(&self) -> Result<Vec<String>, ParseError> {
        let Some(node) = self.doc.first("//h1")? else {
            return Ok(vec![]);
        };
        let text = node.node_text();
        // preg_replace('/^.*"(.*)".*$/', '$1', $text, -1, $count)
        // `.*` is greedy: the *last* pair of quotes wins. Without a complete
        // pair the subject is returned unchanged and `$count === 0`.
        let Some(open) = text.find('"') else {
            return Ok(vec![]);
        };
        let Some(close) = text.rfind('"') else {
            return Ok(vec![]);
        };
        if open == close {
            return Ok(vec![]);
        }
        Ok(text[open + 1..close]
            .split(", ")
            .map(str::to_string)
            .collect())
    }

    /// `CharacterParser::getAbout()`.
    pub fn about(&self) -> Result<Option<String>, ParseError> {
        // MAL wraps the biography in the second table cell; `<br>` is turned
        // into a literal `\n` before re-parsing the fragment so that
        // `removeChildNodes()` keeps the line breaks.
        let Some(cell) = self.doc.first("//*[@id=\"content\"]/table/tr/td[2]")? else {
            return Ok(None);
        };
        let about_html = cell.node_html().replace("<br>", "\\n");
        let fragment = HtmlDoc::parse_str(&format!("<html><body>{about_html}</body></html>"))?;
        let Some(body) = fragment.first("//body")? else {
            return Ok(None);
        };
        body.remove_child_nodes()?;
        let string = cleanse(&body.node_html());
        if string.contains("No biography written.") {
            return Ok(None);
        }
        Ok(Some(string))
    }

    /// `CharacterParser::getMemberFavorites()` (`preg_replace('/\D/', '', ...)`).
    pub fn member_favorites(&self) -> Result<i64, ParseError> {
        let Some(cell) = self.doc.first("//*[@id=\"content\"]/table/tr/td[1]")? else {
            return Ok(0);
        };
        cell.remove_child_nodes()?;
        Ok(digits_only(&cell.node_text()).parse().unwrap_or(0))
    }

    /// `CharacterParser::getImage()` (`og:image`).
    pub fn image(&self) -> Result<Option<String>, ParseError> {
        self.doc.attr("//meta[@property=\"og:image\"]", "content")
    }

    /// `CharacterParser::getAnimeography()`.
    pub fn animeography(&self) -> Result<Vec<Value>, ParseError> {
        let rows = self
            .doc
            .nodes("//div[contains(text(), 'Animeography')]/../table[1]/tr")?;
        rows.iter()
            .map(|row| AnimeographyParser::new(row.clone()).model())
            .collect()
    }

    /// `CharacterParser::getMangaography()`.
    pub fn mangaography(&self) -> Result<Vec<Value>, ParseError> {
        let rows = self
            .doc
            .nodes("//div[contains(text(), 'Mangaography')]/../table[2]/tr")?;
        rows.iter()
            .map(|row| MangaographyParser::new(row.clone()).model())
            .collect()
    }

    /// `CharacterParser::getVoiceActors()`.
    pub fn voice_actors(&self) -> Result<Vec<Value>, ParseError> {
        let rows = self
            .doc
            .nodes("//div[contains(text(), 'Voice Actors')]/../table/tr")?;
        rows.iter()
            .map(|row| VoiceActorParser::new(row.clone()).model())
            .collect()
    }

    /// `CharacterParser::getModel()` (JMS-shaped JSON).
    pub fn model(&self) -> Result<Value, ParseError> {
        // Same call order as `Character::fromParser()`: the ography getters
        // must run before `getMemberFavorites()`, which strips children from
        // `td[1]` and can detach the tables.
        let animeography = self.animeography()?;
        let mangaography = self.mangaography()?;
        let voice_actors = self.voice_actors()?;
        Ok(json!({
            "mal_id": self.mal_id()?,
            "url": self.character_url()?,
            "name": self.name()?,
            "name_kanji": self.name_kanji()?,
            "nicknames": self.nicknames()?,
            "about": self.about()?,
            "member_favorites": self.member_favorites()?,
            "images": character_image_resource(self.image()?.as_deref()),
            "animeography": animeography,
            "mangaography": mangaography,
            "voice_actors": voice_actors,
        }))
    }
}

// ---------------------------------------------------------------------------
// OgraphyParser / AnimeographyParser / MangaographyParser
// ---------------------------------------------------------------------------

/// `Jikan\Parser\Character\OgraphyParser`.
pub struct OgraphyParser {
    node: HtmlNode,
}

impl OgraphyParser {
    pub fn new(node: HtmlNode) -> Self {
        OgraphyParser { node }
    }

    /// `OgraphyParser::getUrl()`.
    pub fn url(&self) -> Result<Option<String>, ParseError> {
        self.node.attr("//td/a", "href")
    }

    /// `OgraphyParser::getName()`.
    pub fn name(&self) -> Result<Option<String>, ParseError> {
        self.node.text("//td/a")
    }

    /// `OgraphyParser::getImage()`.
    pub fn image(&self) -> Result<Option<String>, ParseError> {
        Ok(self
            .node
            .attr("//img", "data-src")?
            .map(|url| parse_image_quality(&url)))
    }

    /// `OgraphyParser::getRole()` (`<small>` last).
    pub fn role(&self) -> Result<Option<String>, ParseError> {
        Ok(self.node.last("//small")?.map(|node| node.node_text()))
    }
}

/// `Jikan\Parser\Character\AnimeographyParser`.
pub struct AnimeographyParser {
    inner: OgraphyParser,
}

impl AnimeographyParser {
    pub fn new(node: HtmlNode) -> Self {
        AnimeographyParser {
            inner: OgraphyParser::new(node),
        }
    }

    /// `AnimeographyParser::getAnimeMeta()`.
    pub fn anime_meta(&self) -> Result<Value, ParseError> {
        Ok(anime_meta(
            &self.inner.name()?.unwrap_or_default(),
            &self.inner.url()?.unwrap_or_default(),
            self.inner.image()?.as_deref(),
        ))
    }

    /// `AnimeographyParser::getModel()`.
    pub fn model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "role": self.inner.role()?.unwrap_or_default(),
            "anime": self.anime_meta()?,
        }))
    }
}

/// `Jikan\Parser\Character\MangaographyParser`.
pub struct MangaographyParser {
    inner: OgraphyParser,
}

impl MangaographyParser {
    pub fn new(node: HtmlNode) -> Self {
        MangaographyParser {
            inner: OgraphyParser::new(node),
        }
    }

    /// `MangaographyParser::getMangaMeta()`.
    pub fn manga_meta(&self) -> Result<Value, ParseError> {
        Ok(manga_meta(
            &self.inner.name()?.unwrap_or_default(),
            &self.inner.url()?.unwrap_or_default(),
            self.inner.image()?.as_deref(),
        ))
    }

    /// `MangaographyParser::getModel()`.
    pub fn model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "role": self.inner.role()?.unwrap_or_default(),
            "manga": self.manga_meta()?,
        }))
    }
}

// ---------------------------------------------------------------------------
// VoiceActorParser
// ---------------------------------------------------------------------------

/// `Jikan\Parser\Character\VoiceActorParser`.
pub struct VoiceActorParser {
    node: HtmlNode,
}

impl VoiceActorParser {
    pub fn new(node: HtmlNode) -> Self {
        VoiceActorParser { node }
    }

    /// `VoiceActorParser::getName()`: first `<a>` without an `<img>` child.
    pub fn name(&self) -> Result<Option<String>, ParseError> {
        Ok(self.person_anchor()?.map(|node| node.node_text()))
    }

    /// `VoiceActorParser::getUrl()`: first `<a>`, image or not.
    pub fn url(&self) -> Result<Option<String>, ParseError> {
        self.node.attr("//a", "href")
    }

    /// `VoiceActorParser::getImage()` (`src ?? data-src`).
    pub fn image(&self) -> Result<Option<String>, ParseError> {
        let Some(img) = self.node.first("//img")? else {
            return Ok(None);
        };
        let src = img.node_attr("src").or_else(|| img.node_attr("data-src"));
        Ok(src.map(|url| parse_image_quality(&url)))
    }

    /// `VoiceActorParser::getLanguage()`.
    pub fn language(&self) -> Result<String, ParseError> {
        if let Some(node) = self
            .node
            .first("//div[contains(@class, \"js-anime-character-language\")]")?
        {
            return Ok(utf8_nbsp_trim(&cleanse(&node.node_text())));
        }
        Ok(self
            .node
            .text("//div/small")?
            .map(|text| utf8_nbsp_trim(&cleanse(&text)))
            .unwrap_or_default())
    }

    /// `VoiceActorParser::getPerson()` (`MalUrl` of the name anchor).
    pub fn person(&self) -> Result<Value, ParseError> {
        let Some(anchor) = self.person_anchor()? else {
            return Ok(json!({
                "mal_id": 0,
                "type": "",
                "name": "",
                "url": "",
            }));
        };
        mal_url(&anchor)
    }

    /// `VoiceActorParser::getPersonMeta()`.
    pub fn person_meta(&self) -> Result<Value, ParseError> {
        Ok(person_meta(
            &self.name()?.unwrap_or_default(),
            &self.url()?.unwrap_or_default(),
            self.image()?.as_deref(),
        ))
    }

    /// `VoiceActorParser::getModel()`.
    pub fn model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "person": self.person_meta()?,
            "language": self.language()?,
        }))
    }

    /// The first `<a>` that does not contain an `<img>` (`->reduce()`).
    fn person_anchor(&self) -> Result<Option<HtmlNode>, ParseError> {
        for anchor in self.node.nodes("//a")? {
            if anchor.nodes("//img")?.is_empty() {
                return Ok(Some(anchor));
            }
        }
        Ok(None)
    }
}

// ---------------------------------------------------------------------------
// CharacterListItemParser
// ---------------------------------------------------------------------------

/// `Jikan\Parser\Character\CharacterListItemParser`.
pub struct CharacterListItemParser {
    node: HtmlNode,
}

impl CharacterListItemParser {
    pub fn new(node: HtmlNode) -> Self {
        CharacterListItemParser { node }
    }

    /// `CharacterListItemParser::getCharacterUrl()`.
    pub fn character_url(&self) -> Result<Option<String>, ParseError> {
        self.node.attr("//td[2]/div[3]/a", "href")
    }

    /// `CharacterListItemParser::getMalId()`.
    pub fn mal_id(&self) -> Result<i64, ParseError> {
        Ok(id_from_url(&self.character_url()?.unwrap_or_default()))
    }

    /// `CharacterListItemParser::getName()`.
    pub fn name(&self) -> Result<Option<String>, ParseError> {
        self.node
            .text("//h3[contains(@class, \"h3_character_name\")]")
    }

    /// `CharacterListItemParser::getImage()`.
    pub fn image(&self) -> Result<Option<String>, ParseError> {
        Ok(self
            .node
            .attr("//img[1]", "data-src")?
            .map(|url| parse_image_quality(&url)))
    }

    /// `CharacterListItemParser::getRole()`.
    pub fn role(&self) -> Result<String, ParseError> {
        Ok(self
            .node
            .text("//td[2]/div[4]")?
            .map(|text| utf8_nbsp_trim(&cleanse(&text)))
            .unwrap_or_default())
    }

    /// `CharacterListItemParser::getFavorites()`.
    pub fn favorites(&self) -> Result<i64, ParseError> {
        Ok(self
            .node
            .text("//td[2]/div[5]")?
            .map(|text| php_int_prefix(&text.replace(',', "")))
            .unwrap_or(0))
    }

    /// `CharacterListItemParser::getVoiceActors()`.
    pub fn voice_actors(&self) -> Result<Vec<Value>, ParseError> {
        let rows = self.node.nodes("//table[2]/tr")?;
        rows.iter()
            .map(|row| VoiceActorParser::new(row.clone()).model())
            .collect()
    }

    /// `CharacterListItemParser::getCharacterMeta()`.
    pub fn character_meta(&self) -> Result<Value, ParseError> {
        Ok(character_meta(
            &self.name()?.unwrap_or_default(),
            &self.character_url()?.unwrap_or_default(),
            self.image()?.as_deref(),
        ))
    }

    /// `CharacterListItemParser::getModel()` (anime/character listing shape).
    pub fn model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "character": self.character_meta()?,
            "role": self.role()?,
            "favorites": self.favorites()?,
            "voice_actors": self.voice_actors()?,
        }))
    }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// `preg_replace('/\D/', '', $input)`.
fn digits_only(input: &str) -> String {
    input.chars().filter(char::is_ascii_digit).collect()
}

/// PHP `(int)` cast on a string: leading whitespace/sign, then digits.
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
