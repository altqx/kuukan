//! Top-list request commands — port of `app/Dto/QueryTop*.php`.

use std::ops::Deref;

use kuukan_core::enums::{
    AnimeRating, AnimeType, MangaType, TopAnimeFilter, TopMangaFilter, TopReviewsType,
};
use kuukan_core::error::ApiError;
use kuukan_core::params::Query;

use super::base::{HasRequestFingerprint, QueryParser, QueryTopItemsCommand};

/// PHP `App\Dto\QueryTopAnimeItemsCommand`.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryTopAnimeItemsCommand {
    pub top: QueryTopItemsCommand,
    pub sfw: bool,
    pub r#type: Option<AnimeType>,
    pub rating: Option<AnimeRating>,
    pub filter: Option<TopAnimeFilter>,
}

impl Deref for QueryTopAnimeItemsCommand {
    type Target = QueryTopItemsCommand;

    fn deref(&self) -> &Self::Target {
        &self.top
    }
}

impl QueryTopAnimeItemsCommand {
    pub fn parse(query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::clean(query);
        let top = QueryTopItemsCommand::parse_with(&mut parser);
        let sfw = parser.bool_flag("sfw", false);
        let anime_type = parser.enum_optional::<AnimeType>("type", AnimeType::PHP_CLASS);
        let rating = parser.enum_optional::<AnimeRating>("rating", AnimeRating::PHP_CLASS);
        let filter = parser.enum_optional::<TopAnimeFilter>("filter", TopAnimeFilter::PHP_CLASS);
        parser.finish()?;
        Ok(Self {
            top: top?,
            sfw,
            r#type: anime_type?,
            rating: rating?,
            filter: filter?,
        })
    }
}

/// PHP `App\Dto\QueryTopMangaItemsCommand`.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryTopMangaItemsCommand {
    pub top: QueryTopItemsCommand,
    pub sfw: bool,
    pub r#type: Option<MangaType>,
    pub filter: Option<TopMangaFilter>,
}

impl Deref for QueryTopMangaItemsCommand {
    type Target = QueryTopItemsCommand;

    fn deref(&self) -> &Self::Target {
        &self.top
    }
}

impl QueryTopMangaItemsCommand {
    pub fn parse(query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::clean(query);
        let top = QueryTopItemsCommand::parse_with(&mut parser);
        let sfw = parser.bool_flag("sfw", false);
        let manga_type = parser.enum_optional::<MangaType>("type", MangaType::PHP_CLASS);
        let filter = parser.enum_optional::<TopMangaFilter>("filter", TopMangaFilter::PHP_CLASS);
        parser.finish()?;
        Ok(Self {
            top: top?,
            sfw,
            r#type: manga_type?,
            filter: filter?,
        })
    }
}

/// PHP `App\Dto\QueryTopCharactersCommand`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryTopCharactersCommand {
    pub top: QueryTopItemsCommand,
}

impl Deref for QueryTopCharactersCommand {
    type Target = QueryTopItemsCommand;

    fn deref(&self) -> &Self::Target {
        &self.top
    }
}

impl QueryTopCharactersCommand {
    pub fn parse(query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::clean(query);
        let top = QueryTopItemsCommand::parse_with(&mut parser);
        parser.finish()?;
        Ok(Self { top: top? })
    }
}

/// PHP `App\Dto\QueryTopPeopleCommand`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryTopPeopleCommand {
    pub top: QueryTopItemsCommand,
}

impl Deref for QueryTopPeopleCommand {
    type Target = QueryTopItemsCommand;

    fn deref(&self) -> &Self::Target {
        &self.top
    }
}

impl QueryTopPeopleCommand {
    pub fn parse(query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::clean(query);
        let top = QueryTopItemsCommand::parse_with(&mut parser);
        parser.finish()?;
        Ok(Self { top: top? })
    }
}

/// PHP `App\Dto\QueryTopReviewsCommand`.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryTopReviewsCommand {
    pub top: QueryTopItemsCommand,
    pub r#type: Option<TopReviewsType>,
    pub preliminary: Option<bool>,
    pub spoilers: Option<bool>,
}

impl Deref for QueryTopReviewsCommand {
    type Target = QueryTopItemsCommand;

    fn deref(&self) -> &Self::Target {
        &self.top
    }
}

impl HasRequestFingerprint for QueryTopReviewsCommand {}

impl QueryTopReviewsCommand {
    pub fn parse(query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::clean(query);
        let top = QueryTopItemsCommand::parse_with(&mut parser);
        let reviews_type =
            parser.enum_optional::<TopReviewsType>("type", TopReviewsType::PHP_CLASS);
        let preliminary = parser.optional_bool_flag("preliminary");
        let spoilers = parser.optional_bool_flag("spoilers");
        parser.finish()?;
        Ok(Self {
            top: top?,
            r#type: reviews_type?,
            preliminary,
            spoilers,
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
    fn top_anime_defaults() {
        let command = QueryTopAnimeItemsCommand::parse(&Query::new()).unwrap();
        assert_eq!(command.page, 1);
        assert_eq!(command.limit, 25);
        assert!(!command.sfw);
        assert_eq!(command.r#type, None);
        assert_eq!(command.filter, None);
    }

    #[test]
    fn top_anime_invalid_fields() {
        let query = Query::from_pairs([
            ("limit", "abc"),
            ("type", "x"),
            ("rating", "y"),
            ("filter", "z"),
            ("sfw", "q"),
        ]);
        let messages = bag(QueryTopAnimeItemsCommand::parse(&query).unwrap_err());
        assert_eq!(
            messages["limit"],
            serde_json::json!([
                "The limit must be a number.",
                "The limit must be an integer.",
                "Value abc is higher than the configured '25' max value."
            ])
        );
        assert_eq!(
            messages["type"],
            serde_json::json!(["The type field is not a valid App\\Enums\\AnimeTypeEnum."])
        );
        assert_eq!(
            messages["rating"],
            serde_json::json!(["The rating field is not a valid App\\Enums\\AnimeRatingEnum."])
        );
        assert_eq!(
            messages["filter"],
            serde_json::json!(["The filter field is not a valid App\\Enums\\TopAnimeFilterEnum."])
        );
        assert_eq!(
            messages["sfw"],
            serde_json::json!(["The sfw field must be true or false."])
        );
    }

    #[test]
    fn top_manga_and_others() {
        let query = Query::from_pairs([("type", "x"), ("filter", "z")]);
        let messages = bag(QueryTopMangaItemsCommand::parse(&query).unwrap_err());
        assert_eq!(
            messages["type"],
            serde_json::json!(["The type field is not a valid App\\Enums\\MangaTypeEnum."])
        );
        assert_eq!(
            messages["filter"],
            serde_json::json!(["The filter field is not a valid App\\Enums\\TopMangaFilterEnum."])
        );

        let query = Query::from_pairs([("limit", "0"), ("page", "x")]);
        let messages = bag(QueryTopCharactersCommand::parse(&query).unwrap_err());
        assert_eq!(
            messages["limit"],
            serde_json::json!(["The limit must be at least 1."])
        );
        assert_eq!(
            messages["page"],
            serde_json::json!(["The page must be a number."])
        );

        assert!(QueryTopPeopleCommand::parse(&Query::new()).is_ok());
    }

    #[test]
    fn top_reviews_invalid_fields() {
        let query = Query::from_pairs([("type", "x"), ("preliminary", "q")]);
        let messages = bag(QueryTopReviewsCommand::parse(&query).unwrap_err());
        assert_eq!(
            messages["type"],
            serde_json::json!(["The type field is not a valid App\\Enums\\TopReviewsTypeEnum."])
        );
        assert_eq!(
            messages["preliminary"],
            serde_json::json!(["The preliminary field must be true or false."])
        );
    }
}
