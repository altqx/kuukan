//! MyAnimeList media URL transforms.
//!
//! Port of the `Media.php` / `Parser.php` string helpers: turning a CDN image
//! URL into its full-quality form, and a YouTube watch/embed URL into an id
//! plus its thumbnail set.
//!
//! These used to sit in `helper.rs` beside the DOM module, which they have
//! nothing to do with: they take a string and return a string, and never touch
//! a document.

use std::sync::OnceLock;

use regex::Regex;
use serde_json::{json, Value};

/// `Parser::parseImageQuality()`.
pub(crate) fn parse_image_quality(image_url: &str) -> String {
    // adding `v` prefix returns a very small thumbnail, as opposed to adding `l`
    let image_url = image_url
        .replace("v.jpg", ".jpg")
        .replace("t.jpg", ".jpg")
        .replace("l.jpg", ".jpg");
    image_quality_re().replace_all(&image_url, "").to_string()
}

/// `Parser::parseImageThumbToHQ()`.
pub(crate) fn parse_image_thumb_to_hq(image_url: &str) -> String {
    image_url.replace("thumbs/", "").replace("_thumb", "")
}

/// `Media::youtubeIdFromUrl()`.
pub(crate) fn youtube_id_from_url(url: Option<&str>) -> Option<String> {
    let url = url?;
    youtube_re().captures(url).map(|caps| caps[1].to_string())
}

/// `Media::generateYoutubeUrlFromId()`.
pub(crate) fn generate_youtube_url_from_id(id: Option<&str>) -> Option<String> {
    id.map(|id| format!("https://www.youtube.com/watch?v={id}"))
}

/// `Media::generateYoutubeImageResource()` + JMS serialization.
pub(crate) fn youtube_image_resource(id: Option<&str>) -> Value {
    match id {
        None => json!({
            "image_url": null,
            "small_image_url": null,
            "medium_image_url": null,
            "large_image_url": null,
            "maximum_image_url": null,
        }),
        Some(id) => json!({
            "image_url": format!("https://img.youtube.com/vi/{id}/default.jpg"),
            "small_image_url": format!("https://img.youtube.com/vi/{id}/sddefault.jpg"),
            "medium_image_url": format!("https://img.youtube.com/vi/{id}/mqdefault.jpg"),
            "large_image_url": format!("https://img.youtube.com/vi/{id}/hqdefault.jpg"),
            "maximum_image_url": format!("https://img.youtube.com/vi/{id}/maxresdefault.jpg"),
        }),
    }
}

fn image_quality_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"/r/\d+x\d+").expect("valid regex"))
}

fn youtube_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"^(?:https?://)?(?:www\.)?(?:m\.)?(?:youtu\.be/|youtube\.com/(?:(?:watch)?\?(?:.*&)?v(?:i)?=|(?:embed|v|vi|user|shorts)/))([^?&"'>]{11})"#,
        )
        .expect("valid regex")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn youtube_id_provider() {
        assert_eq!(
            youtube_id_from_url(Some(
                "https://www.youtube.com/embed/1yXa8MAmocQ?enablejsapi=1&wmode=opaque&autoplay=1"
            ))
            .as_deref(),
            Some("1yXa8MAmocQ")
        );
        assert_eq!(
            youtube_id_from_url(Some(
                "https://www.youtube.com/embed/yhNzL20gNX0/?enablejsapi=1&wmode=opaque&autoplay=1"
            ))
            .as_deref(),
            Some("yhNzL20gNX0")
        );
        assert_eq!(
            youtube_id_from_url(Some("https://youtu.be/dQw4w9WgXcQ")).as_deref(),
            Some("dQw4w9WgXcQ")
        );
        assert_eq!(youtube_id_from_url(None), None);
    }

    #[test]
    fn image_helpers() {
        assert_eq!(
            parse_image_quality("https://cdn.myanimelist.net/images/anime/1/1v.jpg"),
            "https://cdn.myanimelist.net/images/anime/1/1.jpg"
        );
        assert_eq!(
            parse_image_quality("https://cdn.myanimelist.net/r/100x140/images/anime/1/1.jpg"),
            "https://cdn.myanimelist.net/images/anime/1/1.jpg"
        );
        assert_eq!(
            parse_image_thumb_to_hq(
                "https://cdn.myanimelist.net/images/anime/1/thumbs/1_thumb.jpg"
            ),
            "https://cdn.myanimelist.net/images/anime/1/1.jpg"
        );
        assert_eq!(
            generate_youtube_url_from_id(Some("abc")),
            Some("https://www.youtube.com/watch?v=abc".to_string())
        );
        let images = youtube_image_resource(Some("abc"));
        assert_eq!(
            images["image_url"],
            "https://img.youtube.com/vi/abc/default.jpg"
        );
        assert_eq!(
            images["small_image_url"],
            "https://img.youtube.com/vi/abc/sddefault.jpg"
        );
        assert_eq!(
            images["medium_image_url"],
            "https://img.youtube.com/vi/abc/mqdefault.jpg"
        );
        assert_eq!(
            images["large_image_url"],
            "https://img.youtube.com/vi/abc/hqdefault.jpg"
        );
        assert_eq!(
            images["maximum_image_url"],
            "https://img.youtube.com/vi/abc/maxresdefault.jpg"
        );
        assert_eq!(youtube_image_resource(None)["image_url"], Value::Null);
    }
}
