//! Review-list request commands — port of `app/Dto/QueryAnimeReviewsCommand.php`
//! and `QueryMangaReviewsCommand.php`.

use std::ops::Deref;

use kuukan_core::error::ApiError;
use kuukan_core::params::Query;

use super::base::{HasRequestFingerprint, QueryParser, QueryReviewsCommand};

/// PHP `App\Dto\QueryAnimeReviewsCommand`.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryAnimeReviewsCommand {
    pub reviews: QueryReviewsCommand,
}

impl Deref for QueryAnimeReviewsCommand {
    type Target = QueryReviewsCommand;

    fn deref(&self) -> &Self::Target {
        &self.reviews
    }
}

impl HasRequestFingerprint for QueryAnimeReviewsCommand {}

impl QueryAnimeReviewsCommand {
    pub fn parse(query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::clean(query);
        let reviews = QueryReviewsCommand::parse_with(&mut parser);
        parser.finish()?;
        Ok(Self { reviews: reviews? })
    }
}

/// PHP `App\Dto\QueryMangaReviewsCommand`.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryMangaReviewsCommand {
    pub reviews: QueryReviewsCommand,
}

impl Deref for QueryMangaReviewsCommand {
    type Target = QueryReviewsCommand;

    fn deref(&self) -> &Self::Target {
        &self.reviews
    }
}

impl HasRequestFingerprint for QueryMangaReviewsCommand {}

impl QueryMangaReviewsCommand {
    pub fn parse(query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::clean(query);
        let reviews = QueryReviewsCommand::parse_with(&mut parser);
        parser.finish()?;
        Ok(Self { reviews: reviews? })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bag(err: ApiError) -> serde_json::Value {
        err.body(false)["messages"].clone()
    }

    #[test]
    fn reviews_defaults() {
        let command = QueryAnimeReviewsCommand::parse(&Query::new()).unwrap();
        assert_eq!(command.page, 1);
        assert_eq!(command.preliminary, None);
        assert_eq!(command.spoilers, None);
        assert_eq!(command.sort, None);
    }

    #[test]
    fn reviews_values_and_errors() {
        let query = Query::from_pairs([
            ("sort", "newest"),
            ("preliminary", "true"),
            ("spoilers", "false"),
            ("page", "2"),
        ]);
        let command = QueryMangaReviewsCommand::parse(&query).unwrap();
        assert_eq!(command.page, 2);
        assert_eq!(command.preliminary, Some(true));
        assert_eq!(command.spoilers, Some(false));

        let query = Query::from_pairs([
            ("sort", "bogus"),
            ("preliminary", "x"),
            ("spoilers", "y"),
            ("page", "z"),
        ]);
        let messages = bag(QueryAnimeReviewsCommand::parse(&query).unwrap_err());
        assert_eq!(
            messages["sort"],
            serde_json::json!(["The sort field is not a valid App\\Enums\\MediaReviewsSortEnum."])
        );
        assert_eq!(
            messages["preliminary"],
            serde_json::json!(["The preliminary field must be true or false."])
        );
        assert_eq!(
            messages["spoilers"],
            serde_json::json!(["The spoilers field must be true or false."])
        );
        assert_eq!(messages["page"], serde_json::json!(["The page must be a number."]));
    }
}
