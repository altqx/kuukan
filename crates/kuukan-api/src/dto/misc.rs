//! Random request commands — port of `app/Dto/QueryRandom*.php`.

use kuukan_core::error::ApiError;
use kuukan_core::params::Query;

use super::base::QueryParser;

/// PHP `App\Dto\QueryRandomAnimeCommand`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryRandomAnimeCommand {
    pub sfw: bool,
    pub unapproved: bool,
}

impl QueryRandomAnimeCommand {
    pub fn parse(query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::clean(query);
        let sfw = parser.bool_flag("sfw", false);
        let unapproved = parser.bool_flag("unapproved", false);
        parser.finish()?;
        Ok(Self { sfw, unapproved })
    }
}

/// PHP `App\Dto\QueryRandomMangaCommand`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryRandomMangaCommand {
    pub sfw: bool,
    pub unapproved: bool,
}

impl QueryRandomMangaCommand {
    pub fn parse(query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::clean(query);
        let sfw = parser.bool_flag("sfw", false);
        let unapproved = parser.bool_flag("unapproved", false);
        parser.finish()?;
        Ok(Self { sfw, unapproved })
    }
}

/// PHP `App\Dto\QueryRandomCharacterCommand` (no parameters).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryRandomCharacterCommand;

impl QueryRandomCharacterCommand {
    pub fn parse(_query: &Query) -> Result<Self, ApiError> {
        Ok(Self)
    }
}

/// PHP `App\Dto\QueryRandomPersonCommand` (no parameters).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryRandomPersonCommand;

impl QueryRandomPersonCommand {
    pub fn parse(_query: &Query) -> Result<Self, ApiError> {
        Ok(Self)
    }
}

/// PHP `App\Dto\QueryRandomUserCommand` (no parameters).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryRandomUserCommand;

impl QueryRandomUserCommand {
    pub fn parse(_query: &Query) -> Result<Self, ApiError> {
        Ok(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_defaults_and_flags() {
        let command = QueryRandomAnimeCommand::parse(&Query::new()).unwrap();
        assert!(!command.sfw);
        assert!(!command.unapproved);

        let query = Query::from_pairs([("sfw", ""), ("unapproved", "true")]);
        let command = QueryRandomAnimeCommand::parse(&query).unwrap();
        assert!(command.sfw);
        assert!(command.unapproved);

        let query = Query::from_pairs([("sfw", "q")]);
        let err = QueryRandomAnimeCommand::parse(&query).unwrap_err();
        assert_eq!(
            err.body(false)["messages"]["sfw"][0],
            "The sfw field must be true or false."
        );
    }

    #[test]
    fn random_others_ignore_parameters() {
        assert!(QueryRandomCharacterCommand::parse(&Query::from_pairs([("sfw", "x")])).is_ok());
        assert!(QueryRandomPersonCommand::parse(&Query::new()).is_ok());
        assert!(QueryRandomUserCommand::parse(&Query::new()).is_ok());
        assert!(QueryRandomMangaCommand::parse(&Query::new()).is_ok());
    }
}
