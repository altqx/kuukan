//! Seasonal anime request commands — port of `app/Dto/Query*AnimeSeason*.php`.

use std::ops::Deref;

use kuukan_core::enums::AnimeSeason;
use kuukan_core::error::ApiError;
use kuukan_core::params::Query;

use super::base::{HasRequestFingerprint, QueryAnimeSeasonCommand, QueryParser};

/// PHP `App\Dto\QueryAnimeSeasonListCommand` (no parameters).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryAnimeSeasonListCommand;

impl HasRequestFingerprint for QueryAnimeSeasonListCommand {}

impl QueryAnimeSeasonListCommand {
    pub fn parse(_query: &Query) -> Result<Self, ApiError> {
        Ok(Self)
    }
}

/// PHP `App\Dto\QueryCurrentAnimeSeasonCommand`.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryCurrentAnimeSeasonCommand {
    pub season: QueryAnimeSeasonCommand,
}

impl Deref for QueryCurrentAnimeSeasonCommand {
    type Target = QueryAnimeSeasonCommand;

    fn deref(&self) -> &Self::Target {
        &self.season
    }
}

impl HasRequestFingerprint for QueryCurrentAnimeSeasonCommand {}

impl QueryCurrentAnimeSeasonCommand {
    pub fn parse(query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::clean(query);
        let season = QueryAnimeSeasonCommand::parse_with(&mut parser);
        parser.finish()?;
        Ok(Self { season: season? })
    }
}

/// PHP `App\Dto\QueryUpcomingAnimeSeasonCommand`.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryUpcomingAnimeSeasonCommand {
    pub season: QueryAnimeSeasonCommand,
}

impl Deref for QueryUpcomingAnimeSeasonCommand {
    type Target = QueryAnimeSeasonCommand;

    fn deref(&self) -> &Self::Target {
        &self.season
    }
}

impl HasRequestFingerprint for QueryUpcomingAnimeSeasonCommand {}

impl QueryUpcomingAnimeSeasonCommand {
    pub fn parse(query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::clean(query);
        let season = QueryAnimeSeasonCommand::parse_with(&mut parser);
        parser.finish()?;
        Ok(Self { season: season? })
    }
}

/// PHP `App\Dto\QuerySpecificAnimeSeasonCommand` (route
/// `/seasons/{year}/{season}`).
///
/// The limit default in PHP is `protected static int $defaultLimit = 30`, but
/// `max_results_per_page(30)` returns the configured `MAX_RESULTS_PER_PAGE`
/// whenever it is set (it always is, default 25), so the effective default is
/// the global one.
#[derive(Debug, Clone, PartialEq)]
pub struct QuerySpecificAnimeSeasonCommand {
    pub season: QueryAnimeSeasonCommand,
    pub year: i64,
    pub season_name: AnimeSeason,
}

impl Deref for QuerySpecificAnimeSeasonCommand {
    type Target = QueryAnimeSeasonCommand;

    fn deref(&self) -> &Self::Target {
        &self.season
    }
}

impl HasRequestFingerprint for QuerySpecificAnimeSeasonCommand {}

impl QuerySpecificAnimeSeasonCommand {
    pub fn parse(year: i64, season: &str, query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::clean(query);
        let base = QueryAnimeSeasonCommand::parse_with(&mut parser);
        let year = parser.year(year);
        let season_name =
            parser.enum_required::<AnimeSeason>("season", Some(season), AnimeSeason::PHP_CLASS);
        parser.finish()?;
        Ok(Self {
            season: base?,
            year,
            season_name: season_name.expect("validation error was recorded"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bag(err: ApiError) -> serde_json::Value {
        err.body(false)["messages"].clone()
    }

    #[test]
    fn season_defaults() {
        let command = QueryCurrentAnimeSeasonCommand::parse(&Query::new()).unwrap();
        assert_eq!(command.page, 1);
        assert_eq!(command.limit, 25);
        assert!(!command.sfw);
        assert!(!command.unapproved);
        assert_eq!(command.kids, None);
        assert!(!command.continuing);
        assert_eq!(command.filter, None);
    }

    #[test]
    fn season_values() {
        let query = Query::from_pairs([
            ("sfw", "true"),
            ("kids", "true"),
            ("unapproved", "true"),
            ("continuing", "true"),
            ("filter", "tv"),
            ("limit", "10"),
        ]);
        let command = QueryUpcomingAnimeSeasonCommand::parse(&query).unwrap();
        assert!(command.sfw);
        assert_eq!(command.kids, Some(true));
        assert!(command.unapproved);
        assert!(command.continuing);
        assert_eq!(command.filter, Some(kuukan_core::enums::AnimeType::Tv));
        assert_eq!(command.limit, 10);
    }

    #[test]
    fn season_invalid_filter() {
        let query = Query::from_pairs([("filter", "bogus"), ("kids", "x")]);
        let messages = bag(QueryCurrentAnimeSeasonCommand::parse(&query).unwrap_err());
        assert_eq!(
            messages["filter"],
            serde_json::json!(["The filter field is not a valid App\\Enums\\AnimeTypeEnum."])
        );
        assert_eq!(
            messages["kids"],
            serde_json::json!(["The kids field must be true or false."])
        );
    }

    #[test]
    fn specific_season_route_params() {
        let command =
            QuerySpecificAnimeSeasonCommand::parse(2024, "winter", &Query::new()).unwrap();
        assert_eq!(command.year, 2024);
        assert_eq!(command.season_name, AnimeSeason::Winter);
        assert_eq!(command.limit, 25);

        let messages =
            bag(QuerySpecificAnimeSeasonCommand::parse(999, "winter", &Query::new()).unwrap_err());
        assert_eq!(
            messages["year"],
            serde_json::json!(["The year must be between 1000 and 2999."])
        );

        // The PHP `messages()` override for `season.enum` is unreachable:
        // `getFromLocalArray()` is queried with the rule class name, so the
        // default EnumRule string is used.
        let messages =
            bag(QuerySpecificAnimeSeasonCommand::parse(2024, "bogus", &Query::new()).unwrap_err());
        assert_eq!(
            messages["season"],
            serde_json::json!(["The season field is not a valid App\\Enums\\AnimeSeasonEnum."])
        );
    }

    #[test]
    fn season_list_is_empty() {
        assert!(QueryAnimeSeasonListCommand::parse(&Query::new()).is_ok());
    }
}
