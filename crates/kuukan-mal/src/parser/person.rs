//! Port of `Jikan\Parser\Person\*` (jikan-php v4.0.12).

use regex::Regex;
use serde_json::{json, Value};
use std::sync::OnceLock;

use crate::error::ParseError;
use crate::parser::character::{
    anime_meta, character_meta, manga_meta, person_image_resource, user_image_resource,
};
use crate::parser::date::{format_atom, parse_date_mdy_readable};
use crate::parser::helper::{HtmlDoc, HtmlNode};
use crate::parser::jstring::{cleanse, utf8_nbsp_trim};

/// `Jikan\Model\Common\UserMetaBasic` (`{url, username}`).
pub fn user_meta_basic(username: &str, url: &str) -> Value {
    json!({ "url": url, "username": username })
}

/// `Jikan\Model\Common\UserMeta` (`{username, url, images}`).
pub fn user_meta(username: &str, url: &str, image_url: Option<&str>) -> Value {
    json!({
        "username": username,
        "url": url,
        "images": user_image_resource(image_url),
    })
}

fn person_id_from_url(url: &str) -> i64 {
    // PersonParser::getPersonId(): '#https?://myanimelist.net/people/(\d+)#'
    match person_id_re().captures(url) {
        Some(caps) => caps[1].parse().unwrap_or(0),
        None => 0,
    }
}

fn person_id_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"https?://myanimelist\.net/people/(\d+)").expect("valid regex"))
}

fn family_name_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"Family name:(.*?)(Alternate names|Birthday|Website|Member Favorites|More)")
            .expect("valid regex")
    })
}

/// `Jikan\Parser\Person\PersonParser`.
pub struct PersonParser {
    doc: HtmlDoc,
}

impl PersonParser {
    pub fn new(doc: HtmlDoc) -> Self {
        PersonParser { doc }
    }

    /// `PersonParser::getPersonURL()`.
    pub fn person_url(&self) -> Result<Option<String>, ParseError> {
        self.doc.attr("//meta[@property='og:url']", "content")
    }

    /// `PersonParser::getPersonId()`.
    pub fn person_id(&self) -> Result<i64, ParseError> {
        Ok(person_id_from_url(&self.person_url()?.unwrap_or_default()))
    }

    /// `PersonParser::getPersonName()`.
    pub fn person_name(&self) -> Result<Option<String>, ParseError> {
        Ok(self
            .doc
            .attr("//meta[@property='og:title']", "content")?
            .map(|name| cleanse(&name)))
    }

    /// `PersonParser::getPersonImageUrl()`.
    pub fn person_image_url(&self) -> Result<Option<String>, ParseError> {
        self.doc.attr("//meta[@property='og:image']", "content")
    }

    /// `PersonParser::getPersonGivenName()`.
    pub fn given_name(&self) -> Result<Option<String>, ParseError> {
        let Some(node) = self.doc.first("//span[text()=\"Given name:\"]")? else {
            return Ok(None);
        };
        let value = ancestor_text_minus(&node, &node.node_text())
            .map(|text| cleanse(&text))
            .unwrap_or_default();
        Ok(if value.is_empty() { None } else { Some(value) })
    }

    /// `PersonParser::getPersonFamilyName()`.
    pub fn family_name(&self) -> Result<Option<String>, ParseError> {
        let Some(node) = self.doc.first(
            "//div[@id=\"content\"]/table/tr/td[@class=\"borderClass\"]/span[text()=\"Family name:\"]",
        )?
        else {
            return Ok(None);
        };
        let Some(ancestor) = node.ancestors().into_iter().next() else {
            return Ok(None);
        };
        let ancestor_text = ancestor.node_text();
        let Some(caps) = family_name_re().captures(&ancestor_text) else {
            return Ok(None);
        };
        let family = cleanse(&caps[1]);
        Ok(if family.is_empty() { None } else { Some(family) })
    }

    /// `PersonParser::getPersonAlternateNames()`.
    pub fn alternate_names(&self) -> Result<Vec<String>, ParseError> {
        let Some(node) = self.doc.first(
            "//div[@id=\"content\"]/table/tr/td[@class=\"borderClass\"]/div/span[text()=\"Alternate names:\"]",
        )?
        else {
            return Ok(vec![]);
        };
        let Some(ancestor) = node.ancestors().into_iter().next() else {
            return Ok(vec![]);
        };
        let text = ancestor.node_text().replace(&node.node_text(), "");
        Ok(text.split(',').map(|name| cleanse(name)).collect())
    }

    /// `PersonParser::getPersonWebsite()`.
    ///
    /// PHP calls `nextAll()` on a possibly empty crawler and would throw; the
    /// empty case degrades to `null` here (helper convention).
    pub fn website(&self) -> Result<Option<String>, ParseError> {
        let Some(node) = self.doc.first(
            "//div[@id=\"content\"]/table/tr/td[@class=\"borderClass\"]/span[text()=\"Website:\"]",
        )?
        else {
            return Ok(None);
        };
        let mut anchors = Vec::new();
        for sibling in node.next_all() {
            anchors.extend(sibling.css_nodes("a")?);
        }
        let Some(anchor) = anchors.first() else {
            return Ok(None);
        };
        // MAL returns an empty `<a href="http://"></a>` when there's no website.
        if anchor.node_text().is_empty() {
            return Ok(None);
        }
        Ok(anchor.node_attr("href"))
    }

    /// `PersonParser::getPersonBirthday()`.
    pub fn birthday(&self) -> Result<Option<String>, ParseError> {
        let Some(node) = self.doc.first("//span[text()=\"Birthday:\"]")? else {
            return Ok(None);
        };
        let raw = ancestor_text_minus(&node, &node.node_text())
            .map(|text| cleanse(&text))
            .unwrap_or_default();
        Ok(parse_date_mdy_readable(&raw).map(|date| format_atom(&date)))
    }

    /// `PersonParser::getPersonFavorites()`.
    pub fn favorites(&self) -> Result<Option<i64>, ParseError> {
        let Some(node) = self.doc.first("//span[text()=\"Member Favorites:\"]")? else {
            return Ok(None);
        };
        let Some(ancestor) = node.ancestors().into_iter().next() else {
            return Ok(None);
        };
        let text = ancestor
            .node_text()
            .replace(&node.node_text(), "")
            .replace(',', "");
        Ok(Some(php_int_prefix(&cleanse(&text))))
    }

    /// `PersonParser::getPersonAbout()`.
    pub fn about(&self) -> Result<Option<String>, ParseError> {
        let cell = self.doc.first(
            "//div[@id=\"content\"]/table/tr/td[@class=\"borderClass\"]",
        )?;
        let Some(cell) = cell else {
            return Ok(None);
        };
        let Some(node) = cell.css_nodes(".people-informantion-more")?.into_iter().next() else {
            return Ok(None);
        };
        if node.node_text().is_empty() {
            return Ok(None);
        }
        Ok(Some(cleanse(&node.node_html())))
    }

    /// `PersonParser::getPersonVoiceActingRoles()`.
    pub fn voice_acting_roles(&self) -> Result<Vec<Value>, ParseError> {
        let rows = self.doc.nodes(
            "//table[contains(@class, \"js-table-people-character\")]/tr[contains(@class, \"js-people-character\")]",
        )?;
        rows.iter()
            .map(|row| VoiceActingRoleParser::new(row.clone()).model())
            .collect()
    }

    /// `PersonParser::getPersonAnimeStaffPositions()`.
    pub fn anime_staff_positions(&self) -> Result<Vec<Value>, ParseError> {
        let rows = self.doc.nodes(
            "//table[contains(@class, \"js-table-people-staff\")]/tr[contains(@class, \"js-people-staff\")]",
        )?;
        rows.iter()
            .map(|row| AnimeStaffPositionParser::new(row.clone()).model())
            .collect()
    }

    /// `PersonParser::getPersonPublishedManga()`.
    pub fn published_manga(&self) -> Result<Vec<Value>, ParseError> {
        let rows = self.doc.nodes(
            "//table[contains(@class, \"js-table-people-manga\")]/tr[contains(@class, \"js-people-manga\")]",
        )?;
        rows.iter()
            .map(|row| PublishedMangaParser::new(row.clone()).model())
            .collect()
    }

    /// `PersonParser::getModel()`.
    pub fn model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "mal_id": self.person_id()?,
            "url": self.person_url()?,
            "images": person_image_resource(self.person_image_url()?.as_deref()),
            "website_url": self.website()?,
            "name": self.person_name()?,
            "given_name": self.given_name()?,
            "family_name": self.family_name()?,
            "alternate_names": self.alternate_names()?,
            "birthday": self.birthday()?,
            "member_favorites": self.favorites()?.unwrap_or(0),
            "about": self.about()?,
            "voice_acting_roles": self.voice_acting_roles()?,
            "anime_staff_positions": self.anime_staff_positions()?,
            "published_manga": self.published_manga()?,
        }))
    }
}

/// `Jikan\Parser\Person\VoiceActingRoleParser`.
pub struct VoiceActingRoleParser {
    node: HtmlNode,
}

impl VoiceActingRoleParser {
    pub fn new(node: HtmlNode) -> Self {
        VoiceActingRoleParser { node }
    }

    /// `VoiceActingRoleParser::getRole()`.
    pub fn role(&self) -> Result<Option<String>, ParseError> {
        let Some(node) = self.node.first("//td[3]/div[2]")? else {
            return Ok(None);
        };
        Ok(Some(utf8_nbsp_trim(&cleanse(&node.node_text()))))
    }

    /// `VoiceActingRoleParser::getAnimeMeta()`.
    pub fn anime_meta(&self) -> Result<Value, ParseError> {
        let url = self.node.first("//td[2]/div/a")?;
        let image = self.node.first("//td[1]/div/a/img")?;
        Ok(anime_meta(
            &url.as_ref().map(|n| n.node_text()).unwrap_or_default(),
            &url.as_ref().and_then(|n| n.node_attr("href")).unwrap_or_default(),
            image.as_ref().and_then(|n| n.node_attr("data-src")).as_deref(),
        ))
    }

    /// `VoiceActingRoleParser::getCharacterMeta()`.
    pub fn character_meta(&self) -> Result<Value, ParseError> {
        let url = self.node.first("//td[3]/div/a")?;
        let image = self.node.first("//td[4]/div/a/img")?;
        Ok(character_meta(
            &url.as_ref().map(|n| n.node_text()).unwrap_or_default(),
            &url.as_ref().and_then(|n| n.node_attr("href")).unwrap_or_default(),
            image.as_ref().and_then(|n| n.node_attr("data-src")).as_deref(),
        ))
    }

    /// `VoiceActingRoleParser::getModel()`.
    pub fn model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "role": self.role()?,
            "anime": self.anime_meta()?,
            "character": self.character_meta()?,
        }))
    }
}

/// `Jikan\Parser\Person\AnimeStaffPositionParser`.
pub struct AnimeStaffPositionParser {
    node: HtmlNode,
}

impl AnimeStaffPositionParser {
    pub fn new(node: HtmlNode) -> Self {
        AnimeStaffPositionParser { node }
    }

    /// `AnimeStaffPositionParser::getPosition()`.
    pub fn position(&self) -> Result<String, ParseError> {
        Ok(self
            .node
            .text("//td[2]/div[2]")?
            .map(|text| cleanse(&text))
            .unwrap_or_default())
    }

    /// `AnimeStaffPositionParser::getAnimeMeta()`.
    pub fn anime_meta(&self) -> Result<Value, ParseError> {
        let url = self.node.first("//td[position() = 2]/div/a")?;
        let image = self.node.first("//td[position() = 1]/div/a/img")?;
        Ok(anime_meta(
            &url.as_ref().map(|n| n.node_text()).unwrap_or_default(),
            &url.as_ref().and_then(|n| n.node_attr("href")).unwrap_or_default(),
            image.as_ref().and_then(|n| n.node_attr("data-src")).as_deref(),
        ))
    }

    /// `AnimeStaffPositionParser::getModel()`.
    pub fn model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "position": self.position()?,
            "anime": self.anime_meta()?,
        }))
    }
}

/// `Jikan\Parser\Person\PublishedMangaParser`.
pub struct PublishedMangaParser {
    node: HtmlNode,
}

impl PublishedMangaParser {
    pub fn new(node: HtmlNode) -> Self {
        PublishedMangaParser { node }
    }

    /// `PublishedMangaParser::getPosition()`.
    pub fn position(&self) -> Result<String, ParseError> {
        Ok(self
            .node
            .text("//td[2]/div[2]/small")?
            .map(|text| cleanse(&text))
            .unwrap_or_default())
    }

    /// `PublishedMangaParser::getMangaMeta()`.
    pub fn manga_meta(&self) -> Result<Value, ParseError> {
        let url = self.node.first("//td[position() = 2]/div/a")?;
        let image = self.node.first("//td[position() = 1]/div/a/img")?;
        Ok(manga_meta(
            &url.as_ref().map(|n| n.node_text()).unwrap_or_default(),
            &url.as_ref().and_then(|n| n.node_attr("href")).unwrap_or_default(),
            image.as_ref().and_then(|n| n.node_attr("data-src")).as_deref(),
        ))
    }

    /// `PublishedMangaParser::getModel()`.
    pub fn model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "position": self.position()?,
            "manga": self.manga_meta()?,
        }))
    }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// `str_replace($needle, '', $node->ancestors()->text())` — the first
/// (closest) element ancestor's text.
fn ancestor_text_minus(node: &HtmlNode, needle: &str) -> Option<String> {
    node.ancestors()
        .into_iter()
        .next()
        .map(|ancestor| ancestor.node_text().replace(needle, ""))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn php_int_prefix_matches_php_cast() {
        assert_eq!(php_int_prefix("38896"), 38896);
        assert_eq!(php_int_prefix(" 12,474"), 12);
        assert_eq!(php_int_prefix("abc"), 0);
        assert_eq!(php_int_prefix("-12x"), -12);
    }

    #[test]
    fn person_id_regex_allows_http() {
        assert_eq!(
            person_id_from_url("https://myanimelist.net/people/99/Miyuki_Sawashiro"),
            99
        );
        assert_eq!(
            person_id_from_url("http://myanimelist.net/people/99/Miyuki_Sawashiro"),
            99
        );
        assert_eq!(person_id_from_url("https://myanimelist.net/anime/99"), 0);
    }

    #[test]
    fn parse_image_style_webp_urls() {
        // Guard the null webp quirk replicated across the meta builders.
        let value = user_image_resource(Some("https://x/1.jpg"));
        assert_eq!(
            value["webp"]["image_url"],
            Value::String("https://x/1.webp".into())
        );
        let value = person_image_resource(None);
        assert!(value["jpg"]["image_url"].is_null());
    }

    #[test]
    fn image_quality_applied_in_meta() {
        let value = anime_meta(
            "T",
            "https://myanimelist.net/anime/1",
            Some("https://cdn.myanimelist.net/r/84x124/images/anime/1/2.jpg?s=x"),
        );
        assert_eq!(
            value["images"]["jpg"]["image_url"],
            Value::String("https://cdn.myanimelist.net/images/anime/1/2.jpg?s=x".into())
        );
    }
}
