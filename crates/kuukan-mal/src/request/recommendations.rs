//! Port of `Jikan\Request\Recommendations\RecentRecommendationsRequest`.

use crate::request::{http_build_query, MalRequest, QueryValue, BASE_URL};

/// `Constants::RECENT_RECOMMENDATION_ANIME` / `..._MANGA`.
pub const VALID_RECOMMENDATION_TYPES: [&str; 2] = ["anime", "manga"];

/// `Jikan\Request\Recommendations\RecentRecommendationsRequest`.
#[derive(Debug, Clone)]
pub struct RecentRecommendationsRequest {
    type_: String,
    page: u64,
}

impl RecentRecommendationsRequest {
    /// `new RecentRecommendationsRequest($type = 'anime', $page = 1)`.
    pub fn new(type_: &str, page: u64) -> Result<Self, String> {
        if !VALID_RECOMMENDATION_TYPES.contains(&type_) {
            return Err(format!("Recommendation type {type_} is not valid"));
        }
        Ok(RecentRecommendationsRequest {
            type_: type_.to_string(),
            page,
        })
    }

    /// `getPage()`.
    pub fn page(&self) -> u64 {
        self.page
    }
}

impl MalRequest for RecentRecommendationsRequest {
    fn path(&self) -> String {
        let query = http_build_query(&[
            ("s", QueryValue::Str("recentrecs")),
            ("t", QueryValue::Str(&self.type_)),
            (
                "show",
                if self.page != 1 {
                    QueryValue::UInt(100 * (self.page - 1))
                } else {
                    QueryValue::None
                },
            ),
        ]);
        format!("{BASE_URL}/recommendations.php?{query}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_matches_php() {
        assert_eq!(
            RecentRecommendationsRequest::new("anime", 1).unwrap().path(),
            "https://myanimelist.net/recommendations.php?s=recentrecs&t=anime"
        );
        assert_eq!(
            RecentRecommendationsRequest::new("manga", 3).unwrap().path(),
            "https://myanimelist.net/recommendations.php?s=recentrecs&t=manga&show=200"
        );
        assert!(RecentRecommendationsRequest::new("novel", 1).is_err());
    }
}
