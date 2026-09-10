//! Port of `Jikan\Parser\Club\*` (jikan-php v4.0.12).

use regex::Regex;
use serde_json::{json, Value};
use std::sync::OnceLock;

use crate::error::ParseError;
use crate::parser::character::wrap_image_resource;
use crate::parser::date::{format_atom, parse_date};
use crate::parser::helper::{parse_image_quality, parse_image_thumb_to_hq, HtmlDoc};
use crate::parser::jstring::cleanse;
use crate::parser::mal_url::{club_id_from_url, MalUrl};

/// `Jikan\Model\Common\UserMetaBasic` (`{url, username}`).
fn user_meta_basic(username: &str, url: &str) -> Value {
    json!({ "url": url, "username": username })
}

/// `Jikan\Parser\Club\ClubParser`.
pub struct ClubParser {
    doc: HtmlDoc,
}

impl ClubParser {
    pub fn new(doc: HtmlDoc) -> Self {
        ClubParser { doc }
    }

    /// `ClubParser::getUrl()`.
    pub fn url(&self) -> Result<String, ParseError> {
        Ok(self
            .doc
            .attr("//meta[@property='og:url']", "content")?
            .unwrap_or_default())
    }

    /// `ClubParser::getMalId()`.
    pub fn mal_id(&self) -> Result<i64, ParseError> {
        Ok(club_id_from_url(&self.url()?))
    }

    /// `ClubParser::getImageUrl()`.
    pub fn image_url(&self) -> Result<String, ParseError> {
        Ok(self
            .doc
            .attr(
                "//div[@id=\"content\"]/table/tr/td[2]/div/div[1]/img",
                "data-src",
            )?
            .map(|url| parse_image_quality(&url))
            .unwrap_or_default())
    }

    /// `ClubParser::getTitle()`.
    pub fn title(&self) -> Result<String, ParseError> {
        Ok(self
            .doc
            .text("//div[@id=\"contentWrapper\"]/div[1]/h1")?
            .unwrap_or_default())
    }

    /// `ClubParser::getMembersCount()`.
    pub fn members_count(&self) -> Result<i64, ParseError> {
        self.int_from_div(4)
    }

    /// `ClubParser::getPicturesCount()`.
    pub fn pictures_count(&self) -> Result<i64, ParseError> {
        self.int_from_div(5)
    }

    /// `ClubParser::getCategory()`.
    pub fn category(&self) -> Result<String, ParseError> {
        let Some(node) = self
            .doc
            .first("//div[@id=\"content\"]/table/tr/td[2]/div/div[6]")?
        else {
            return Ok(String::new());
        };
        node.remove_child_nodes()?;
        Ok(cleanse(&node.node_text()).to_lowercase())
    }

    /// `ClubParser::getCreated()`.
    pub fn created(&self) -> Result<Option<String>, ParseError> {
        let Some(node) = self
            .doc
            .first("//div[@id=\"content\"]/table/tr/td[2]/div/div[contains(., \"Created\")]")?
        else {
            return Ok(None);
        };
        node.remove_child_nodes()?;
        let raw = cleanse(&node.node_text());
        Ok(parse_date(&raw).map(|date| format_atom(&date)))
    }

    /// `ClubParser::getType()` (serialized as `access`).
    pub fn access(&self) -> Result<String, ParseError> {
        let Some(node) = self
            .doc
            .first("//div[@id=\"content\"]/table/tr/td[2]/div")?
        else {
            return Ok(String::new());
        };
        let text = cleanse(&node.node_text());
        Ok(club_type_re()
            .captures(&text)
            .map(|caps| caps[1].to_string())
            .unwrap_or_default())
    }

    /// `ClubParser::getAnimeRelations()`.
    pub fn anime_relations(&self) -> Result<Vec<Value>, ParseError> {
        self.relations("Anime")
    }

    /// `ClubParser::getMangaRelations()`.
    pub fn manga_relations(&self) -> Result<Vec<Value>, ParseError> {
        self.relations("Manga")
    }

    /// `ClubParser::getCharacterRelations()`.
    pub fn character_relations(&self) -> Result<Vec<Value>, ParseError> {
        self.relations("Character")
    }

    /// `ClubParser::getStaff()`.
    pub fn staff(&self) -> Result<Vec<Value>, ParseError> {
        let Some(header) = self.doc.first(
            "//div[contains(text(), \"Club Staff\") and @class=\"normal_header\"]",
        )?
        else {
            return Ok(vec![]);
        };

        let mut anchors = Vec::new();
        for sibling in header.next_all() {
            anchors.extend(sibling.nodes("//a")?);
        }

        let mut staff = Vec::new();
        for anchor in anchors {
            let href = anchor.node_attr("href").unwrap_or_default();
            if !profile_re().is_match(&href) {
                continue;
            }
            staff.push(user_meta_basic(
                &anchor.node_text(),
                &format!("{}{href}", crate::request::BASE_URL),
            ));
        }
        Ok(staff)
    }

    /// `ClubParser::getModel()`.
    pub fn model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "mal_id": self.mal_id()?,
            "url": self.url()?,
            "images": wrap_image_resource(Some(&self.image_url()?)),
            "name": self.title()?,
            "members": self.members_count()?,
            "category": self.category()?,
            "created": self.created()?,
            "access": self.access()?,
            "staff": self.staff()?,
            "anime": self.anime_relations()?,
            "manga": self.manga_relations()?,
            "characters": self.character_relations()?,
        }))
    }

    fn int_from_div(&self, index: usize) -> Result<i64, ParseError> {
        let Some(node) = self.doc.first(&format!(
            "//div[@id=\"content\"]/table/tr/td[2]/div/div[{index}]"
        ))?
        else {
            return Ok(0);
        };
        node.remove_child_nodes()?;
        Ok(digits_only(&node.node_text()))
    }

    /// The shared Anime/Manga/Character relations branch.
    ///
    /// PHP builds the URL as `BASE_URL . '/' . href`; MAL's club links are
    /// relative (`anime/1`) so the result has no slug.
    fn relations(&self, kind: &str) -> Result<Vec<Value>, ParseError> {
        let Some(header) = self.doc.first(&format!(
            "//div[text()=\"{kind} Relations\" and @class=\"normal_header\"]"
        ))?
        else {
            return Ok(vec![]);
        };

        let mut anchors = Vec::new();
        for sibling in header.next_all() {
            anchors.extend(sibling.nodes("//a")?);
        }

        let pattern = format!(r"{}/\d+", kind.to_lowercase());
        let regex = Regex::new(&pattern).expect("valid relation regex");
        let mut relations = Vec::new();
        for anchor in anchors {
            let href = anchor.node_attr("href").unwrap_or_default();
            if !regex.is_match(&href) {
                continue;
            }
            relations.push(
                MalUrl::new(
                    anchor.node_text(),
                    format!("{}/{href}", crate::request::BASE_URL),
                )
                .to_json(),
            );
        }
        Ok(relations)
    }
}

/// `Jikan\Parser\Club\UserListParser`.
pub struct UserListParser {
    doc: HtmlDoc,
}

impl UserListParser {
    pub fn new(doc: HtmlDoc) -> Self {
        UserListParser { doc }
    }

    /// `UserListParser::getResults()`.
    pub fn results(&self) -> Result<Vec<Value>, ParseError> {
        self.doc
            .nodes("//*[@id=\"content\"]/table/tr/td")?
            .iter()
            .map(|node| UserProfileParser::new(node.clone()).model())
            .collect()
    }

    /// `UserListParser::hasNextPage()`.
    pub fn has_next_page(&self) -> Result<bool, ParseError> {
        if self
            .doc
            .count("//*[@id=\"content\"]/div/a[contains(., \"Last\")]")?
            > 0
        {
            return Ok(true);
        }
        Ok(self
            .doc
            .count("//*[@id=\"content\"]/div/a[contains(., \"»\")]")?
            > 0)
    }

    /// `UserListParser::getLastPage()`.
    pub fn last_page(&self) -> Result<i64, ParseError> {
        let Some(node) = self.doc.first("//*[@id=\"content\"]/div")? else {
            return Ok(1);
        };
        Ok(pages_re()
            .captures(&node.node_text())
            .and_then(|caps| caps[1].parse().ok())
            .unwrap_or(1))
    }

    /// `UserListParser::getModel()`.
    pub fn model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "results": self.results()?,
            "has_next_page": self.has_next_page()?,
            "last_visible_page": self.last_page()?,
        }))
    }
}

/// `Jikan\Parser\Club\UserProfileParser`.
pub struct UserProfileParser {
    node: crate::parser::helper::HtmlNode,
}

impl UserProfileParser {
    pub fn new(node: crate::parser::helper::HtmlNode) -> Self {
        UserProfileParser { node }
    }

    /// `UserProfileParser::getUsername()`.
    pub fn username(&self) -> Result<String, ParseError> {
        Ok(self.node.text("//a[1]")?.unwrap_or_default())
    }

    /// `UserProfileParser::getUrl()`.
    pub fn url(&self) -> Result<String, ParseError> {
        Ok(format!(
            "{}{}",
            crate::request::BASE_URL,
            self.node.attr("//a[1]", "href")?.unwrap_or_default()
        ))
    }

    /// `UserProfileParser::getImage()`.
    pub fn image(&self) -> Result<String, ParseError> {
        let image_url = self
            .node
            .attr("//img[1]", "data-src")?
            .map(|url| parse_image_thumb_to_hq(&url))
            .unwrap_or_default();

        if image_url.contains(crate::parser::mal_url::CDN_URL) {
            return Ok(image_url);
        }
        if !image_url.contains(crate::request::BASE_URL) {
            return Ok(format!("{}{image_url}", crate::request::BASE_URL));
        }
        Ok(image_url)
    }

    /// `UserProfileParser::getModel()`.
    pub fn model(&self) -> Result<Value, ParseError> {
        let image = self.image()?;
        Ok(json!({
            "username": self.username()?,
            "url": self.url()?,
            "images": crate::parser::character::user_image_resource(Some(&image)),
        }))
    }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn digits_only(input: &str) -> i64 {
    let digits: String = input.chars().filter(char::is_ascii_digit).collect();
    digits.parse().unwrap_or(0)
}

fn club_type_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"This is a (.*?) club\.").expect("valid regex"))
}

fn profile_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"/profile/(.*?)").expect("valid regex"))
}

fn pages_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"Pages \((.*)\)").expect("valid regex"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn club_relations_and_staff_use_relative_hrefs() {
        let doc = HtmlDoc::parse_str(
            r#"
            <div id="contentWrapper"><div><h1>Cowboy Bebop</h1></div>
            <div id="content"><table><tr><td class="borderClass">x</td>
            <td valign="top"><div>
              <div><img data-src="https://cdn.myanimelist.net/images/clubs/16/222057.jpg"></div>
              <div id="profileRows"></div>
              <div class="normal_header">Club Stats</div>
              <div class="spaceit_pad"><span class="dark_text">Members:</span> 1,398</div>
              <div class="spaceit_pad"><span class="dark_text">Pictures:</span> 25</div>
              <div class="spaceit_pad"><span class="dark_text">Category:</span> Anime</div>
              <div class="spaceit_pad"><span class="dark_text">Created:</span> Mar 29, 2007</div>
              <div class="normal_header">Club Staff</div><div><a href="/profile/Xinil">Xinil</a> (President)</div>
              <div class="normal_header">Anime Relations</div><div><a href="anime/1">Cowboy Bebop</a></div>
              <div class="normal_header">Manga Relations</div><div><a href="manga/173">Cowboy Bebop</a></div>
              <div class="normal_header">Character Relations</div><div><a href="character/4">Ein</a></div>
              <div>This is a public club.</div>
            </div></td></tr></table></div></div>
            <meta property="og:url" content="https://myanimelist.net/clubs.php?cid=1">
            "#,
        )
        .unwrap();
        let parser = ClubParser::new(doc);
        assert_eq!(parser.mal_id().unwrap(), 1);
        assert_eq!(parser.members_count().unwrap(), 1398);
        assert_eq!(parser.access().unwrap(), "public");
        assert_eq!(parser.created().unwrap().as_deref(), Some("2007-03-29T00:00:00+00:00"));
        assert_eq!(parser.anime_relations().unwrap()[0]["url"], "https://myanimelist.net/anime/1");
        assert_eq!(parser.anime_relations().unwrap()[0]["mal_id"], 1);
        assert_eq!(parser.staff().unwrap()[0]["username"], "Xinil");
    }
}
