//! User request commands — port of `app/Dto/User*.php`, `UsersSearchCommand`,
//! `QueryAnimeListOfUserCommand` and `QueryMangaListOfUserCommand`.
//!
//! The list commands take the `{status}` route parameter explicitly through
//! `parse_with_status`; the convenience `parse(username, query)` reads it from
//! the query string (PHP maps the route parameter only when the route value is
//! present, so `?status=` works too).

use std::ops::Deref;

use kuukan_core::enums::{
    AnimeListAiringStatusFilter, AnimeListStatus, Gender, MangaListStatus,
    UserAnimeListOrderBy, UserHistoryType, UserMangaListOrderBy,
    UserMangaListStatusFilter,
};
use kuukan_core::error::ApiError;
use kuukan_core::params::Query;

use super::base::{
    id_lookup_command, username_lookup_command, username_page_lookup_command, DateOnly,
    HasRequestFingerprint, QueryListOfUserCommand, QueryParser, SearchCommand,
};

/// PHP `App\Dto\UsersSearchCommand` (uses `PreparesData` through
/// `SearchCommand`; the search handler does **not** run the `q`
/// control-character check).
#[derive(Debug, Clone, PartialEq)]
pub struct UsersSearchCommand {
    pub search: SearchCommand,
    pub min_age: Option<i64>,
    pub max_age: Option<i64>,
    pub gender: Option<Gender>,
    pub location: Option<String>,
}

impl Deref for UsersSearchCommand {
    type Target = SearchCommand;

    fn deref(&self) -> &Self::Target {
        &self.search
    }
}

impl HasRequestFingerprint for UsersSearchCommand {}

impl UsersSearchCommand {
    pub fn parse(query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::clean(query);
        let search = SearchCommand::parse_with(&mut parser);
        let min_age = parser.numeric_int("minAge");
        let max_age = parser.numeric_int("maxAge");
        let gender = parser.enum_optional::<Gender>("gender", Gender::PHP_CLASS);
        let location = parser.optional_string("location");
        parser.finish()?;
        Ok(Self {
            search: search?,
            min_age: min_age?,
            max_age: max_age?,
            gender: gender?,
            location,
        })
    }
}

id_lookup_command! {
    /// PHP `App\Dto\UserByIdLookupCommand`.
    UserByIdLookupCommand
}

username_lookup_command! {
    /// PHP `App\Dto\UserAboutLookupCommand`.
    UserAboutLookupCommand
}

username_lookup_command! {
    /// PHP `App\Dto\UserFullLookupCommand`.
    UserFullLookupCommand
}

username_lookup_command! {
    /// PHP `App\Dto\UserProfileLookupCommand`.
    UserProfileLookupCommand
}

username_lookup_command! {
    /// PHP `App\Dto\UserStatisticsLookupCommand`.
    UserStatisticsLookupCommand
}

username_lookup_command! {
    /// PHP `App\Dto\UserFavoritesLookupCommand`.
    UserFavoritesLookupCommand
}

username_lookup_command! {
    /// PHP `App\Dto\UserExternalLookupCommand`.
    UserExternalLookupCommand
}

username_lookup_command! {
    /// PHP `App\Dto\UserUpdatesLookupCommand`.
    UserUpdatesLookupCommand
}

username_lookup_command! {
    /// PHP `App\Dto\UserClubsLookupCommand`.
    UserClubsLookupCommand
}

username_page_lookup_command! {
    /// PHP `App\Dto\UserFriendsLookupCommand`.
    UserFriendsLookupCommand
}

username_page_lookup_command! {
    /// PHP `App\Dto\UserReviewsLookupCommand`.
    UserReviewsLookupCommand
}

username_page_lookup_command! {
    /// PHP `App\Dto\UserRecommendationsLookupCommand`.
    UserRecommendationsLookupCommand
}

/// PHP `App\Dto\UserHistoryLookupCommand` (nullable `?UserHistoryTypeEnum
/// $type`; the `{type}` route parameter wins over `?type=`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserHistoryLookupCommand {
    pub username: String,
    pub r#type: Option<UserHistoryType>,
}

impl HasRequestFingerprint for UserHistoryLookupCommand {}

impl UserHistoryLookupCommand {
    pub fn parse(username: &str, query: &Query) -> Result<Self, ApiError> {
        Self::parse_with_type(username, None, query)
    }

    pub fn parse_with_type(
        username: &str,
        history_type: Option<&str>,
        query: &Query,
    ) -> Result<Self, ApiError> {
        let mut parser = QueryParser::new(query);
        let username = parser.username_lookup(username);
        let raw = history_type.or_else(|| parser.get("type"));
        let history_type = parser.enum_nullable_raw::<UserHistoryType>(
            "type",
            raw,
            UserHistoryType::PHP_CLASS,
        );
        parser.finish()?;
        Ok(Self {
            username,
            r#type: history_type?,
        })
    }
}

/// PHP `App\Dto\QueryAnimeListOfUserCommand`.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryAnimeListOfUserCommand {
    pub list: QueryListOfUserCommand,
    pub status: Option<AnimeListStatus>,
    pub order_by: Option<UserAnimeListOrderBy>,
    pub order_by2: Option<UserAnimeListOrderBy>,
    pub airing_status: Option<AnimeListAiringStatusFilter>,
    pub year: Option<i64>,
    pub producer: Option<i64>,
    pub aired_from: Option<DateOnly>,
    pub aired_to: Option<DateOnly>,
}

impl Deref for QueryAnimeListOfUserCommand {
    type Target = QueryListOfUserCommand;

    fn deref(&self) -> &Self::Target {
        &self.list
    }
}

impl HasRequestFingerprint for QueryAnimeListOfUserCommand {}

impl QueryAnimeListOfUserCommand {
    /// `{status}` route parameter wins over `?status=`; `None` reads the
    /// query.
    pub fn parse(username: &str, query: &Query) -> Result<Self, ApiError> {
        Self::parse_with_status(username, None, query)
    }

    pub fn parse_with_status(
        username: &str,
        status: Option<&str>,
        query: &Query,
    ) -> Result<Self, ApiError> {
        let mut parser = QueryParser::new(query);
        let list = QueryListOfUserCommand::parse_with(&mut parser, username);
        let raw = status.or_else(|| parser.get("status"));
        let status_value = parser.enum_optional_raw::<AnimeListStatus>(
            "status",
            raw,
            AnimeListStatus::PHP_CLASS,
        );
        let order_by = parser.enum_optional::<UserAnimeListOrderBy>(
            "order_by",
            UserAnimeListOrderBy::PHP_CLASS,
        );
        let order_by2 = parser.enum_optional::<UserAnimeListOrderBy>(
            "order_by2",
            UserAnimeListOrderBy::PHP_CLASS,
        );
        let airing_status = parser.enum_optional::<AnimeListAiringStatusFilter>(
            "airing_status",
            AnimeListAiringStatusFilter::PHP_CLASS,
        );
        let year = parser.int_min_max("year", 1500.0, 2999.0);
        let producer = parser.int_min("producer", 1.0);
        let (aired_from, aired_to) =
            parser.date_range("aired_from", "aired_to");
        parser.finish()?;
        Ok(Self {
            list: list?,
            status: status_value?,
            order_by: order_by?,
            order_by2: order_by2?,
            airing_status: airing_status?,
            year: year?,
            producer: producer?,
            aired_from,
            aired_to,
        })
    }
}

/// PHP `App\Dto\QueryMangaListOfUserCommand`.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryMangaListOfUserCommand {
    pub list: QueryListOfUserCommand,
    pub status: Option<MangaListStatus>,
    pub order_by: Option<UserMangaListOrderBy>,
    pub order_by2: Option<UserMangaListOrderBy>,
    pub magazine: Option<i64>,
    pub published_from: Option<DateOnly>,
    pub published_to: Option<DateOnly>,
    pub publishing_status: Option<UserMangaListStatusFilter>,
}

impl Deref for QueryMangaListOfUserCommand {
    type Target = QueryListOfUserCommand;

    fn deref(&self) -> &Self::Target {
        &self.list
    }
}

impl HasRequestFingerprint for QueryMangaListOfUserCommand {}

impl QueryMangaListOfUserCommand {
    pub fn parse(username: &str, query: &Query) -> Result<Self, ApiError> {
        Self::parse_with_status(username, None, query)
    }

    pub fn parse_with_status(
        username: &str,
        status: Option<&str>,
        query: &Query,
    ) -> Result<Self, ApiError> {
        let mut parser = QueryParser::new(query);
        let list = QueryListOfUserCommand::parse_with(&mut parser, username);
        let raw = status.or_else(|| parser.get("status"));
        let status_value = parser.enum_optional_raw::<MangaListStatus>(
            "status",
            raw,
            MangaListStatus::PHP_CLASS,
        );
        let order_by = parser.enum_optional::<UserMangaListOrderBy>(
            "order_by",
            UserMangaListOrderBy::PHP_CLASS,
        );
        let order_by2 = parser.enum_optional::<UserMangaListOrderBy>(
            "order_by2",
            UserMangaListOrderBy::PHP_CLASS,
        );
        let magazine = parser.int_min("magazine", 1.0);
        let (published_from, published_to) =
            parser.date_range("published_from", "published_to");
        let publishing_status = parser.enum_optional::<UserMangaListStatusFilter>(
            "publishing_status",
            UserMangaListStatusFilter::PHP_CLASS,
        );
        parser.finish()?;
        Ok(Self {
            list: list?,
            status: status_value?,
            order_by: order_by?,
            order_by2: order_by2?,
            magazine: magazine?,
            published_from,
            published_to,
            publishing_status: publishing_status?,
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
    fn users_search_defaults_and_values() {
        let command = UsersSearchCommand::parse(&Query::new()).unwrap();
        assert_eq!(command.page, 1);
        assert_eq!(command.limit, 25);
        assert_eq!(command.min_age, None);
        assert_eq!(command.gender, None);

        let query = Query::from_pairs([
            ("q", "neko"),
            ("minAge", "18"),
            ("maxAge", "30"),
            ("gender", "nonbinary"),
            ("location", "japan"),
        ]);
        let command = UsersSearchCommand::parse(&query).unwrap();
        assert_eq!(command.min_age, Some(18));
        assert_eq!(command.max_age, Some(30));
        assert_eq!(command.gender, Some(Gender::Nonbinary));
        assert_eq!(command.location.as_deref(), Some("japan"));
    }

    #[test]
    fn users_search_camel_case_messages() {
        let query = Query::from_pairs([("minAge", "abc"), ("gender", "x")]);
        let messages = bag(UsersSearchCommand::parse(&query).unwrap_err());
        assert_eq!(
            messages["minAge"],
            serde_json::json!(["The min age must be a number."])
        );
        assert_eq!(
            messages["gender"],
            serde_json::json!(["The gender field is not a valid App\\Enums\\GenderEnum."])
        );
    }

    #[test]
    fn user_lookup_username_rules() {
        let command = UserAboutLookupCommand::parse("nekomata", &Query::new()).unwrap();
        assert_eq!(command.username, "nekomata");

        let messages =
            bag(UserAboutLookupCommand::parse("ab", &Query::new()).unwrap_err());
        assert_eq!(
            messages["username"],
            serde_json::json!(["The username must be at least 3 characters."])
        );

        let long = "a".repeat(256);
        let messages = bag(UserAboutLookupCommand::parse(&long, &Query::new()).unwrap_err());
        assert_eq!(
            messages["username"],
            serde_json::json!(["The username must not be greater than 255 characters."])
        );
    }

    #[test]
    fn user_history_type() {
        let command = UserHistoryLookupCommand::parse("nekomata", &Query::new()).unwrap();
        assert_eq!(command.r#type, None);

        let query = Query::from_pairs([("type", "anime")]);
        let command = UserHistoryLookupCommand::parse("nekomata", &query).unwrap();
        assert_eq!(command.r#type, Some(UserHistoryType::Anime));

        let query = Query::from_pairs([("type", "bogus")]);
        let messages = bag(UserHistoryLookupCommand::parse("nekomata", &query).unwrap_err());
        assert_eq!(
            messages["type"],
            serde_json::json!(["The type field is not a valid App\\Enums\\UserHistoryTypeEnum."])
        );
    }

    #[test]
    fn animelist_defaults_and_status() {
        let command =
            QueryAnimeListOfUserCommand::parse("nekomata", &Query::new()).unwrap();
        assert_eq!(command.page, 1);
        assert_eq!(command.status, None);
        assert_eq!(command.aired_from, None);

        let command = QueryAnimeListOfUserCommand::parse_with_status(
            "nekomata",
            Some("watching"),
            &Query::new(),
        )
        .unwrap();
        assert_eq!(command.status, Some(AnimeListStatus::Watching));

        // `?status=` also works when the route parameter is absent.
        let query = Query::from_pairs([("status", "completed")]);
        let command = QueryAnimeListOfUserCommand::parse("nekomata", &query).unwrap();
        assert_eq!(command.status, Some(AnimeListStatus::Completed));
    }

    #[test]
    fn animelist_invalid_fields() {
        let query = Query::from_pairs([
            ("year", "abc"),
            ("producer", "abc"),
            ("order_by", "bogus"),
        ]);
        let messages =
            bag(QueryAnimeListOfUserCommand::parse("nekomata", &query).unwrap_err());
        assert_eq!(
            messages["year"],
            serde_json::json!([
                "The year must be a number.",
                "The year must be at least 1500."
            ])
        );
        assert_eq!(
            messages["producer"],
            serde_json::json!(["The producer must be a number."])
        );
        assert_eq!(
            messages["order_by"],
            serde_json::json!([
                "The order by field is not a valid App\\Enums\\UserAnimeListOrderByEnum."
            ])
        );
    }

    #[test]
    fn animelist_airing_status_never_resolves() {
        let query = Query::from_pairs([("airing_status", "airing")]);
        let error = QueryAnimeListOfUserCommand::parse("nekomata", &query).unwrap_err();
        // The enum cannot resolve in PHP: `EnumRule` fails, but building the
        // message throws `DuplicateLabelsException`, which escapes as an
        // unhandled 500 `Exception`.
        assert_eq!(error.status(), 500);
        assert_eq!(error.body(false)["type"], "Exception");
    }

    #[test]
    fn animelist_date_range() {
        let query = Query::from_pairs([
            ("aired_from", "2021-01-01"),
            ("aired_to", "2020-01-01"),
        ]);
        let messages =
            bag(QueryAnimeListOfUserCommand::parse("nekomata", &query).unwrap_err());
        assert_eq!(
            messages["aired_from"],
            serde_json::json!([
                "The aired from must be a date before or equal to aired to."
            ])
        );
        assert_eq!(
            messages["aired_to"],
            serde_json::json!([
                "The aired to must be a date after or equal to aired from."
            ])
        );
    }

    #[test]
    fn animelist_empty_date_is_required() {
        let query = Query::from_pairs([("aired_from", "")]);
        let messages =
            bag(QueryAnimeListOfUserCommand::parse("nekomata", &query).unwrap_err());
        assert_eq!(
            messages["aired_from"],
            serde_json::json!(["The aired from field is required."])
        );
        assert!(messages.get("aired_to").is_none());

        let query = Query::from_pairs([("aired_from", ""), ("aired_to", "2010-01-01")]);
        let messages =
            bag(QueryAnimeListOfUserCommand::parse("nekomata", &query).unwrap_err());
        assert_eq!(
            messages["aired_from"],
            serde_json::json!(["The aired from field is required."])
        );
        assert_eq!(
            messages["aired_to"],
            serde_json::json!([
                "The aired to must be a date after or equal to aired from."
            ])
        );
    }

    #[test]
    fn animelist_empty_int_fails_the_cast() {
        let query = Query::from_pairs([("year", "")]);
        let error = QueryAnimeListOfUserCommand::parse("nekomata", &query).unwrap_err();
        assert_eq!(error.status(), 500);
    }

    #[test]
    fn mangalist_broken_enums_are_500() {
        for (field, value) in [("order_by", "title"), ("publishing_status", "publishing")] {
            let query = Query::from_pairs([(field, value)]);
            let error = QueryMangaListOfUserCommand::parse("nekomata", &query).unwrap_err();
            assert_eq!(error.status(), 500, "{field}");
            assert_eq!(error.body(false)["type"], "Exception", "{field}");
        }
    }

    #[test]
    fn mangalist_fields() {
        let query = Query::from_pairs([("magazine", "abc")]);
        let messages =
            bag(QueryMangaListOfUserCommand::parse("nekomata", &query).unwrap_err());
        assert_eq!(
            messages["magazine"],
            serde_json::json!(["The magazine must be a number."])
        );

        let query = Query::from_pairs([
            ("published_from", "2021-01-01"),
            ("published_to", "2020-01-01"),
        ]);
        let messages =
            bag(QueryMangaListOfUserCommand::parse("nekomata", &query).unwrap_err());
        assert_eq!(
            messages["published_from"],
            serde_json::json!([
                "The published from must be a date before or equal to published to."
            ])
        );
    }
}
