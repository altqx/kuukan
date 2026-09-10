//! Watch request commands — port of `app/Dto/QueryPopular*.php`,
//! `QueryRecentlyAdded*.php` and `QueryRecentlyOnlineUsersCommand.php`.

use kuukan_core::error::ApiError;
use kuukan_core::params::Query;

use super::base::{HasRequestFingerprint, QueryParser};

/// PHP `App\Dto\QueryRecentlyAddedEpisodesCommand`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryRecentlyAddedEpisodesCommand;

impl HasRequestFingerprint for QueryRecentlyAddedEpisodesCommand {}

impl QueryRecentlyAddedEpisodesCommand {
    pub fn parse(_query: &Query) -> Result<Self, ApiError> {
        Ok(Self)
    }
}

/// PHP `App\Dto\QueryPopularEpisodesCommand`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryPopularEpisodesCommand;

impl HasRequestFingerprint for QueryPopularEpisodesCommand {}

impl QueryPopularEpisodesCommand {
    pub fn parse(_query: &Query) -> Result<Self, ApiError> {
        Ok(Self)
    }
}

/// PHP `App\Dto\QueryRecentlyAddedPromoVideosCommand` (`page` only; the class
/// does not use `PreparesData`, so an empty page fails the int cast in PHP).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryRecentlyAddedPromoVideosCommand {
    pub page: u64,
}

impl HasRequestFingerprint for QueryRecentlyAddedPromoVideosCommand {}

impl QueryRecentlyAddedPromoVideosCommand {
    pub fn parse(query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::new(query);
        let page = parser.page()?;
        parser.finish()?;
        Ok(Self { page })
    }
}

/// PHP `App\Dto\QueryPopularPromoVideosCommand`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryPopularPromoVideosCommand;

impl HasRequestFingerprint for QueryPopularPromoVideosCommand {}

impl QueryPopularPromoVideosCommand {
    pub fn parse(_query: &Query) -> Result<Self, ApiError> {
        Ok(Self)
    }
}

/// PHP `App\Dto\QueryRecentlyOnlineUsersCommand`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryRecentlyOnlineUsersCommand;

impl QueryRecentlyOnlineUsersCommand {
    pub fn parse(_query: &Query) -> Result<Self, ApiError> {
        Ok(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watch_defaults() {
        assert!(QueryRecentlyAddedEpisodesCommand::parse(&Query::new()).is_ok());
        assert!(QueryPopularEpisodesCommand::parse(&Query::new()).is_ok());
        assert!(QueryPopularPromoVideosCommand::parse(&Query::new()).is_ok());
        assert!(QueryRecentlyOnlineUsersCommand::parse(&Query::new()).is_ok());
        assert_eq!(
            QueryRecentlyAddedPromoVideosCommand::parse(&Query::new())
                .unwrap()
                .page,
            1
        );
    }

    #[test]
    fn watch_promo_page_validation() {
        let query = Query::from_pairs([("page", "x")]);
        let err = QueryRecentlyAddedPromoVideosCommand::parse(&query).unwrap_err();
        assert_eq!(
            err.body(false)["messages"]["page"][0],
            "The page must be a number."
        );
    }
}
