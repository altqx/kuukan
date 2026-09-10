//! Port of `Jikan\Request\Reviews\ReviewsRequest`.

use crate::request::{http_build_query, MalRequest, QueryValue, BASE_URL};

/// `Constants::ANIME` / `Constants::MANGA`.
pub const VALID_REVIEW_TYPES: [&str; 2] = ["anime", "manga"];

/// `Jikan\Request\Reviews\ReviewsRequest`.
#[derive(Debug, Clone)]
pub struct ReviewsRequest {
    type_: String,
    page: u64,
    sort: String,
    spoilers: bool,
    preliminary: bool,
}

impl ReviewsRequest {
    /// `new ReviewsRequest($type = 'anime', $page = 1, $sort = 'mostvoted',
    /// $spoilers = true, $preliminary = true)`.
    pub fn new(
        type_: &str,
        page: u64,
        sort: &str,
        spoilers: bool,
        preliminary: bool,
    ) -> Result<Self, String> {
        if !VALID_REVIEW_TYPES.contains(&type_) {
            return Err(format!("Review type {type_} is not valid"));
        }
        Ok(ReviewsRequest {
            type_: type_.to_string(),
            page,
            sort: sort.to_string(),
            spoilers,
            preliminary,
        })
    }

    /// `getPage()`.
    pub fn page(&self) -> u64 {
        self.page
    }
}

impl MalRequest for ReviewsRequest {
    fn path(&self) -> String {
        let query = http_build_query(&[
            ("p", QueryValue::UInt(self.page)),
            ("t", QueryValue::Str(&self.type_)),
            (
                "spoiler",
                QueryValue::Str(if self.spoilers { "on" } else { "off" }),
            ),
            (
                "preliminary",
                QueryValue::Str(if self.preliminary { "on" } else { "off" }),
            ),
            ("sort", QueryValue::Str(&self.sort)),
        ]);
        format!("{BASE_URL}/reviews.php?{query}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_matches_php() {
        let request = ReviewsRequest::new("anime", 1, "mostvoted", true, true).unwrap();
        assert_eq!(
            request.path(),
            "https://myanimelist.net/reviews.php?p=1&t=anime&spoiler=on&preliminary=on&sort=mostvoted"
        );
        let request = ReviewsRequest::new("manga", 2, "newest", false, false).unwrap();
        assert_eq!(
            request.path(),
            "https://myanimelist.net/reviews.php?p=2&t=manga&spoiler=off&preliminary=off&sort=newest"
        );
        assert!(ReviewsRequest::new("lightnovel", 1, "mostvoted", true, true).is_err());
    }
}
