//! Ports of `Jikan\Parser\Watch\*`.

use serde_json::{json, Value};

use crate::error::ParseError;
use crate::parser::helper::{parse_image_quality, youtube_id_from_url, HtmlDoc, HtmlNode};
use crate::parser::mal_url::{id_from_url, suffix_id_from_url, BASE_URL};

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

/// `Jikan\Model\Common\AnimeMeta` constructor.
fn anime_meta(title: &str, url: &str, image_url: &str) -> Value {
    let image = parse_image_quality(image_url);
    json!({
        "mal_id": id_from_url(url),
        "url": url,
        "images": common_image_resource(Some(&image)),
        "title": title,
    })
}

/// `Jikan\Model\Common\YoutubeMeta::factory()`.
fn youtube_meta(embed_url: Option<&str>) -> Value {
    let youtube_id = youtube_id_from_url(embed_url);
    let url = youtube_id
        .as_deref()
        .map(|id| format!("https://www.youtube.com/watch?v={id}"));
    json!({
        "youtube_id": youtube_id,
        "url": url,
        "embed_url": embed_url,
        "images": crate::parser::helper::youtube_image_resource(youtube_id.as_deref()),
    })
}

// ---------------------------------------------------------------------------
// Episodes
// ---------------------------------------------------------------------------

/// `Jikan\Parser\Watch\WatchEpisodesParser`.
pub struct WatchEpisodesParser<'a> {
    doc: &'a HtmlDoc,
}

impl<'a> WatchEpisodesParser<'a> {
    pub fn new(doc: &'a HtmlDoc) -> Self {
        WatchEpisodesParser { doc }
    }

    /// `Episodes::fromParser()`.
    pub fn get_model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "results": self.get_results()?,
            "has_next_page": false,
            "last_visible_page": 1,
        }))
    }

    /// `WatchEpisodesParser::getResults()`.
    pub fn get_results(&self) -> Result<Vec<Value>, ParseError> {
        let mut out = Vec::new();
        for node in self.doc.nodes(
            "//*[@id=\"content\"]/div[3]/div/div[contains(@class, \"video-list-outer-vertical\")]",
        )? {
            out.push(EpisodeListItemParser::new(&node).get_model()?);
        }
        Ok(out)
    }
}

/// `Jikan\Parser\Watch\EpisodeListItemParser`.
pub struct EpisodeListItemParser<'a> {
    node: &'a HtmlNode,
}

impl<'a> EpisodeListItemParser<'a> {
    pub fn new(node: &'a HtmlNode) -> Self {
        EpisodeListItemParser { node }
    }

    /// `EpisodeListItem::fromParser()`.
    pub fn get_model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "entry": self.get_anime_meta()?,
            "episodes": self.get_episodes()?,
            "region_locked": self.get_region_locked()?,
        }))
    }

    /// `EpisodeListItemParser::getId()`.
    pub fn get_id(&self) -> Result<i64, ParseError> {
        Ok(id_from_url(&self.get_url()?))
    }

    /// `EpisodeListItemParser::getUrl()`.
    pub fn get_url(&self) -> Result<String, ParseError> {
        Ok(self
            .node
            .attr("//div[@class=\"video-info-title\"]/a[2]", "href")?
            .unwrap_or_default())
    }

    /// `EpisodeListItemParser::getTitle()`.
    pub fn get_title(&self) -> Result<String, ParseError> {
        Ok(self
            .node
            .text("//div[@class=\"video-info-title\"]/a[2]")?
            .unwrap_or_default())
    }

    /// `EpisodeListItemParser::getImageUrl()`.
    pub fn get_image_url(&self) -> Result<String, ParseError> {
        let src = self
            .node
            .attr("//div[contains(@class, \"video-list\")]/img", "data-src")?
            .unwrap_or_default();
        Ok(parse_image_quality(&src))
    }

    /// `EpisodeListItemParser::getImages()`.
    pub fn get_images(&self) -> Result<String, ParseError> {
        self.get_image_url()
    }

    /// `EpisodeListItemParser::getEpisodes()`.
    pub fn get_episodes(&self) -> Result<Vec<Value>, ParseError> {
        let mut out = Vec::new();
        for node in self.node.nodes(
            "//div[contains(@class, \"video-list\")]/div[contains(@class, \"info-container\")]/div[contains(@class, \"title\")]/a",
        )? {
            let href = node.node_attr("href").unwrap_or_default();
            let premium = node.count("//span[contains(@class, \"icon-pay\")]")? > 0;
            out.push(json!({
                "mal_id": suffix_id_from_url(&href),
                "url": href,
                "title": node.node_text(),
                "premium": premium,
            }));
        }
        Ok(out)
    }

    /// `EpisodeListItemParser::getRegionLocked()`.
    pub fn get_region_locked(&self) -> Result<bool, ParseError> {
        Ok(self
            .node
            .count("//div[contains(@class, \"is_blocked\")]")?
            > 0)
    }

    /// `EpisodeListItemParser::getAnimeMeta()`.
    pub fn get_anime_meta(&self) -> Result<Value, ParseError> {
        Ok(anime_meta(
            &self.get_title()?,
            &self.get_url()?,
            &self.get_image_url()?,
        ))
    }
}

// ---------------------------------------------------------------------------
// Promotional videos
// ---------------------------------------------------------------------------

/// `Jikan\Parser\Watch\WatchPromotionalVideosParser`.
pub struct WatchPromotionalVideosParser<'a> {
    doc: &'a HtmlDoc,
}

impl<'a> WatchPromotionalVideosParser<'a> {
    pub fn new(doc: &'a HtmlDoc) -> Self {
        WatchPromotionalVideosParser { doc }
    }

    /// `PromotionalVideos::fromParser()`.
    pub fn get_model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "results": self.get_results()?,
            "has_next_page": self.get_has_next_page()?,
            "last_visible_page": self.get_last_visible_page()?,
        }))
    }

    /// `WatchPromotionalVideosParser::getResults()`.
    pub fn get_results(&self) -> Result<Vec<Value>, ParseError> {
        let mut out = Vec::new();
        for node in self.doc.nodes(
            "//*[@id=\"content\"]/div[3]/div/div[contains(@class, \"video-list-outer-vertical\")]",
        )? {
            out.push(PromotionalVideoListItemParser::new(&node).get_model()?);
        }
        Ok(out)
    }

    /// `WatchPromotionalVideosParser::getHasNextPage()`.
    pub fn get_has_next_page(&self) -> Result<bool, ParseError> {
        Ok(self.doc.count(
            "//*[@id=\"content\"]/div[contains(@class, \"pagination\")]/a[contains(text(), \"More\")]",
        )? > 0)
    }

    /// `WatchPromotionalVideosParser::getLastVisiblePage()`.
    pub fn get_last_visible_page(&self) -> Result<i64, ParseError> {
        let Some(node) = self.doc.first(
            "//*[@id=\"content\"]/div[contains(@class, \"pagination\")]/span[@class=\"link-blue-box\"]",
        )? else {
            return Ok(1);
        };
        let current_page: i64 = node.node_text().trim().parse().unwrap_or(0);
        if !self.get_has_next_page()? {
            return Ok(current_page);
        }
        Ok(current_page + 1)
    }
}

/// `Jikan\Parser\Watch\PromotionalVideoListItemParser`.
pub struct PromotionalVideoListItemParser<'a> {
    node: &'a HtmlNode,
}

impl<'a> PromotionalVideoListItemParser<'a> {
    pub fn new(node: &'a HtmlNode) -> Self {
        PromotionalVideoListItemParser { node }
    }

    /// `PromotionalVideoListItem::fromParser()`.
    pub fn get_model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "title": self.get_promo_title()?,
            "entry": anime_meta(&self.get_title()?, &self.get_url()?, &self.get_images()?),
            "trailer": youtube_meta(Some(&self.get_promo_media()?)),
        }))
    }

    /// `PromotionalVideoListItemParser::getId()`.
    pub fn get_id(&self) -> Result<i64, ParseError> {
        Ok(id_from_url(&self.get_url()?))
    }

    /// `PromotionalVideoListItemParser::getUrl()`.
    pub fn get_url(&self) -> Result<String, ParseError> {
        Ok(self
            .node
            .attr("//div[@class=\"video-info-title\"]/a[2]", "href")?
            .unwrap_or_default())
    }

    /// `PromotionalVideoListItemParser::getTitle()`.
    pub fn get_title(&self) -> Result<String, ParseError> {
        Ok(self
            .node
            .text("//div[@class=\"video-info-title\"]/a[2]")?
            .unwrap_or_default())
    }

    /// `PromotionalVideoListItemParser::getImageUrl()`.
    pub fn get_image_url(&self) -> Result<String, ParseError> {
        let src = self
            .node
            .attr("//div[contains(@class, \"video-list\")]/a", "data-bg")?
            .unwrap_or_default();
        Ok(parse_image_quality(&src))
    }

    /// `PromotionalVideoListItemParser::getImages()`.
    pub fn get_images(&self) -> Result<String, ParseError> {
        self.get_image_url()
    }

    /// `PromotionalVideoListItemParser::getPromoMedia()`.
    pub fn get_promo_media(&self) -> Result<String, ParseError> {
        Ok(self
            .node
            .attr("//div[contains(@class, \"video-list\")]/a", "href")?
            .unwrap_or_default())
    }

    /// `PromotionalVideoListItemParser::getPromoTitle()`.
    pub fn get_promo_title(&self) -> Result<String, ParseError> {
        Ok(self
            .node
            .text(
                "//div[contains(@class, \"video-list\")]/a/div[contains(@class, \"info-container\")]/span",
            )?
            .unwrap_or_default())
    }
}

/// The watch pages sometimes carry relative entry hrefs; keep the PHP
/// `Constants::BASE_URL` prefix helper around for callers that need it.
pub fn absolute_url(href: &str) -> String {
    if href.starts_with("http") {
        href.to_string()
    } else {
        format!("{BASE_URL}{href}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::helper::HtmlDoc;

    #[test]
    fn parses_recent_episode_item() {
        let doc = HtmlDoc::parse_str(
            r#"
            <div id="content">
              <div></div><div></div>
              <div><div>
                <div class="video-list-outer-vertical">
                  <div class="video-list">
                    <img data-src="https://cdn.myanimelist.net/images/anime/1/1.jpg" />
                    <div class="info-container">
                      <div class="title">
                        <a href="https://myanimelist.net/anime/1/Test/episode/3">Episode 3<span class="icon-pay"></span></a>
                      </div>
                    </div>
                  </div>
                  <div class="video-info-title">
                    <a href="/anime/1/Test">Test Anime</a>
                    <a href="/anime/1/Test">Test Anime</a>
                  </div>
                  <div class="is_blocked"></div>
                </div>
              </div></div>
            </div>
            "#,
        )
        .unwrap();
        let parser = WatchEpisodesParser::new(&doc);
        let model = parser.get_model().unwrap();
        assert_eq!(model["results"][0]["entry"]["title"], "Test Anime");
        assert_eq!(model["results"][0]["region_locked"], true);
        assert_eq!(model["results"][0]["episodes"][0]["mal_id"], 3);
        assert_eq!(model["results"][0]["episodes"][0]["premium"], true);
    }

    #[test]
    fn parses_promotional_video_item() {
        let doc = HtmlDoc::parse_str(
            r#"
            <div id="content">
              <div></div><div></div>
              <div><div>
                <div class="video-list-outer-vertical">
                  <div class="video-list">
                    <a href="https://www.youtube.com/watch?v=abcdefghijk" data-bg="https://cdn.myanimelist.net/images/anime/2/2.jpg">
                      <div class="info-container"><span>PV 1</span></div>
                    </a>
                  </div>
                  <div class="video-info-title">
                    <a href="/anime/2/Test">Test</a>
                    <a href="/anime/2/Test">Test</a>
                  </div>
                </div>
              </div></div>
            </div>
            "#,
        )
        .unwrap();
        let parser = WatchPromotionalVideosParser::new(&doc);
        let model = parser.get_model().unwrap();
        assert_eq!(model["results"][0]["title"], "PV 1");
        assert_eq!(model["results"][0]["entry"]["title"], "Test");
        assert_eq!(model["results"][0]["trailer"]["youtube_id"], "abcdefghijk");
        assert_eq!(
            model["results"][0]["trailer"]["url"],
            "https://www.youtube.com/watch?v=abcdefghijk"
        );
    }
}
