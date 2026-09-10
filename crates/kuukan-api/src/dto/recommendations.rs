//! Recommendation request commands — port of
//! `app/Dto/QueryAnimeRecommendationsCommand.php` and
//! `QueryMangaRecommendationsCommand.php`.

use kuukan_core::error::ApiError;
use kuukan_core::params::Query;

use super::base::HasRequestFingerprint;

/// PHP `App\Dto\QueryAnimeRecommendationsCommand` (no parameters).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryAnimeRecommendationsCommand;

impl HasRequestFingerprint for QueryAnimeRecommendationsCommand {}

impl QueryAnimeRecommendationsCommand {
    pub fn parse(_query: &Query) -> Result<Self, ApiError> {
        Ok(Self)
    }
}

/// PHP `App\Dto\QueryMangaRecommendationsCommand` (no parameters).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryMangaRecommendationsCommand;

impl HasRequestFingerprint for QueryMangaRecommendationsCommand {}

impl QueryMangaRecommendationsCommand {
    pub fn parse(_query: &Query) -> Result<Self, ApiError> {
        Ok(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recommendations_ignore_query_parameters() {
        let query = Query::from_pairs([("page", "x")]);
        assert!(QueryAnimeRecommendationsCommand::parse(&query).is_ok());
        assert!(QueryMangaRecommendationsCommand::parse(&query).is_ok());
    }
}
