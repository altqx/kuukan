//! Port of `Jikan\Request\Seasonal\SeasonalRequest`.

use crate::request::{MalRequest, BASE_URL};

/// Valid `season` values (PHP `in_array($season, [...], true)`).
pub const VALID_SEASONS: [&str; 4] = ["winter", "spring", "summer", "fall"];

/// `Jikan\Request\Seasonal\SeasonalRequest`.
#[derive(Debug, Clone, Default)]
pub struct SeasonalRequest {
    year: Option<u32>,
    season: Option<String>,
    later: bool,
}

impl SeasonalRequest {
    /// `new SeasonalRequest($year = null, $season = null, $later = false)`.
    ///
    /// PHP throws `InvalidArgumentException` for an unknown season; Kuukan
    /// surfaces that as `Err` so callers can map it to a validation error.
    pub fn new(year: Option<u32>, season: Option<&str>, later: bool) -> Result<Self, String> {
        if let Some(season) = season {
            if !VALID_SEASONS.contains(&season) {
                return Err(format!("Season {season} is not valid"));
            }
        }
        Ok(SeasonalRequest {
            year,
            season: season.map(|s| s.to_string()),
            later,
        })
    }

    /// `getYear()`.
    pub fn year(&self) -> Option<u32> {
        self.year
    }

    /// `getSeason()`.
    pub fn season(&self) -> Option<&str> {
        self.season.as_deref()
    }

    /// `isLater()`.
    pub fn is_later(&self) -> bool {
        self.later
    }
}

impl MalRequest for SeasonalRequest {
    fn path(&self) -> String {
        if self.later {
            return format!("{BASE_URL}/anime/season/later");
        }
        match (self.year, &self.season) {
            (Some(year), Some(season)) => format!("{BASE_URL}/anime/season/{year}/{season}"),
            _ => format!("{BASE_URL}/anime/season"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_match_php() {
        assert_eq!(
            SeasonalRequest::new(Some(2018), Some("spring"), false)
                .unwrap()
                .path(),
            "https://myanimelist.net/anime/season/2018/spring"
        );
        assert_eq!(
            SeasonalRequest::new(None, None, false).unwrap().path(),
            "https://myanimelist.net/anime/season"
        );
        assert_eq!(
            SeasonalRequest::new(None, None, true).unwrap().path(),
            "https://myanimelist.net/anime/season/later"
        );
        assert!(SeasonalRequest::new(Some(2020), Some("monsoon"), false).is_err());
    }
}
