//! Anime request commands — port of `app/Dto/Anime*.php`.
//!
//! Every command exposes:
//!
//! * `AnimeSearchCommand::parse(&Query)`;
//! * `AnimeLookupCommand::parse(id, &Query)` (route id commands);
//! * `AnimeEpisodeLookupCommand::parse(id, episode_id, &Query)`;
//! * `AnimeGenreListCommand::parse(&Query)`.
//!
//! Validation mirrors the PHP DTOs exactly: see [`crate::dto::base`] for the
//! pipeline and message conventions.

use std::ops::Deref;

use kuukan_core::enums::{AnimeForumFilter, AnimeOrderBy, AnimeRating, AnimeStatus, AnimeType};
use kuukan_core::error::ApiError;
use kuukan_core::params::Query;

use super::base::{
    check_search_q, id_lookup_command, id_page_lookup_command, GenreListCommand,
    HasRequestFingerprint, MediaSearchCommand, QueryParser, QueryReviewsCommand,
};

// ---------------------------------------------------------------------------
// Search
// ---------------------------------------------------------------------------

/// PHP `App\Dto\AnimeSearchCommand`.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimeSearchCommand {
    pub media: MediaSearchCommand,
    pub status: Option<AnimeStatus>,
    pub r#type: Option<AnimeType>,
    pub rating: Option<AnimeRating>,
    pub producer: Option<i64>,
    pub producers: Option<String>,
    pub order_by: Option<AnimeOrderBy>,
}

impl Deref for AnimeSearchCommand {
    type Target = MediaSearchCommand;

    fn deref(&self) -> &Self::Target {
        &self.media
    }
}

impl AnimeSearchCommand {
    pub fn parse(query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::clean(query);
        let media = MediaSearchCommand::parse_with(&mut parser);
        let status =
            parser.enum_optional::<AnimeStatus>("status", AnimeStatus::PHP_CLASS);
        let anime_type = parser.enum_optional::<AnimeType>("type", AnimeType::PHP_CLASS);
        let rating =
            parser.enum_optional::<AnimeRating>("rating", AnimeRating::PHP_CLASS);
        let producer = parser.int_min_integer("producer", 1.0);
        let producers = parser.optional_string("producers");
        parser.prohibits("producers", &["producer"]);
        let order_by = parser.enum_optional::<AnimeOrderBy>(
            "order_by",
            AnimeOrderBy::PHP_CLASS,
        );
        parser.finish()?;
        let media = media?;
        let command = Self {
            media,
            status: status?,
            r#type: anime_type?,
            rating: rating?,
            producer: producer?,
            producers,
            order_by: order_by?,
        };
        check_search_q(command.media.search.q.as_deref())?;
        Ok(command)
    }
}

// ---------------------------------------------------------------------------
// Lookups
// ---------------------------------------------------------------------------

id_lookup_command! {
    /// PHP `App\Dto\AnimeLookupCommand`.
    AnimeLookupCommand
}

id_lookup_command! {
    /// PHP `App\Dto\AnimeFullLookupCommand`.
    AnimeFullLookupCommand
}

id_lookup_command! {
    /// PHP `App\Dto\AnimeCharactersLookupCommand`.
    AnimeCharactersLookupCommand
}

id_lookup_command! {
    /// PHP `App\Dto\AnimeStaffLookupCommand`.
    AnimeStaffLookupCommand
}

id_page_lookup_command! {
    /// PHP `App\Dto\AnimeEpisodesLookupCommand` (`page` query parameter).
    AnimeEpisodesLookupCommand
}

/// PHP `App\Dto\AnimeEpisodeLookupCommand` (route id + `episodeId`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimeEpisodeLookupCommand {
    pub id: i64,
    pub episode_id: i64,
}

impl HasRequestFingerprint for AnimeEpisodeLookupCommand {}

impl AnimeEpisodeLookupCommand {
    pub fn parse(id: i64, episode_id: i64, query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::new(query);
        let id = parser.id(id);
        let episode_id = parser.episode_id(episode_id);
        parser.finish()?;
        Ok(Self { id, episode_id })
    }
}

id_page_lookup_command! {
    /// PHP `App\Dto\AnimeNewsLookupCommand`.
    AnimeNewsLookupCommand
}

/// PHP `App\Dto\AnimeForumLookupCommand` (`filter` query parameter).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimeForumLookupCommand {
    pub id: i64,
    pub filter: Option<AnimeForumFilter>,
}

impl HasRequestFingerprint for AnimeForumLookupCommand {}

impl AnimeForumLookupCommand {
    pub fn parse(id: i64, query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::new(query);
        let id = parser.id(id);
        let filter = parser.enum_optional::<AnimeForumFilter>(
            "filter",
            AnimeForumFilter::PHP_CLASS,
        );
        parser.finish()?;
        Ok(Self { id, filter: filter? })
    }
}

id_lookup_command! {
    /// PHP `App\Dto\AnimeVideosLookupCommand`.
    AnimeVideosLookupCommand
}

id_page_lookup_command! {
    /// PHP `App\Dto\AnimeVideosEpisodesLookupCommand`.
    AnimeVideosEpisodesLookupCommand
}

id_lookup_command! {
    /// PHP `App\Dto\AnimePicturesLookupCommand`.
    AnimePicturesLookupCommand
}

id_lookup_command! {
    /// PHP `App\Dto\AnimeStatsLookupCommand`.
    AnimeStatsLookupCommand
}

id_lookup_command! {
    /// PHP `App\Dto\AnimeMoreInfoLookupCommand`.
    AnimeMoreInfoLookupCommand
}

id_lookup_command! {
    /// PHP `App\Dto\AnimeRecommendationsLookupCommand`.
    AnimeRecommendationsLookupCommand
}

id_page_lookup_command! {
    /// PHP `App\Dto\AnimeUserUpdatesLookupCommand`.
    AnimeUserUpdatesLookupCommand
}

/// PHP `App\Dto\AnimeReviewsLookupCommand` (carries `PreparesData`).
#[derive(Debug, Clone, PartialEq)]
pub struct AnimeReviewsLookupCommand {
    pub id: i64,
    pub reviews: QueryReviewsCommand,
}

impl Deref for AnimeReviewsLookupCommand {
    type Target = QueryReviewsCommand;

    fn deref(&self) -> &Self::Target {
        &self.reviews
    }
}

impl HasRequestFingerprint for AnimeReviewsLookupCommand {}

impl AnimeReviewsLookupCommand {
    pub fn parse(id: i64, query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::clean(query);
        let id = parser.id(id);
        let reviews = QueryReviewsCommand::parse_with(&mut parser);
        parser.finish()?;
        Ok(Self {
            id,
            reviews: reviews?,
        })
    }
}

id_lookup_command! {
    /// PHP `App\Dto\AnimeRelationsLookupCommand`.
    AnimeRelationsLookupCommand
}

id_lookup_command! {
    /// PHP `App\Dto\AnimeThemesLookupCommand`.
    AnimeThemesLookupCommand
}

id_lookup_command! {
    /// PHP `App\Dto\AnimeExternalLookupCommand`.
    AnimeExternalLookupCommand
}

id_lookup_command! {
    /// PHP `App\Dto\AnimeStreamingLookupCommand`.
    AnimeStreamingLookupCommand
}

// ---------------------------------------------------------------------------
// Genres
// ---------------------------------------------------------------------------

/// PHP `App\Dto\AnimeGenreListCommand` (`filter` query parameter).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimeGenreListCommand {
    pub genres: GenreListCommand,
}

impl Deref for AnimeGenreListCommand {
    type Target = GenreListCommand;

    fn deref(&self) -> &Self::Target {
        &self.genres
    }
}

impl AnimeGenreListCommand {
    pub fn parse(query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::new(query);
        let genres = GenreListCommand::parse_with(&mut parser);
        parser.finish()?;
        Ok(Self { genres: genres? })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bag(err: ApiError) -> serde_json::Value {
        err.body(false)["messages"].clone()
    }

    #[test]
    fn anime_search_defaults() {
        let command = AnimeSearchCommand::parse(&Query::new()).unwrap();
        assert_eq!(command.q, None);
        assert_eq!(command.sort, None);
        assert_eq!(command.letter, None);
        assert_eq!(command.page, 1);
        assert_eq!(command.limit, 25);
        assert!(!command.sfw);
        assert!(!command.unapproved);
        assert_eq!(command.min_score, None);
        assert_eq!(command.max_score, None);
        assert_eq!(command.score, None);
        assert_eq!(command.start_date, None);
        assert_eq!(command.end_date, None);
        assert_eq!(command.status, None);
        assert_eq!(command.r#type, None);
        assert_eq!(command.rating, None);
        assert_eq!(command.producer, None);
        assert_eq!(command.producers, None);
        assert_eq!(command.order_by, None);
    }

    #[test]
    fn anime_search_accepts_values() {
        let query = Query::from_pairs([
            ("q", "naruto"),
            ("sort", "asc"),
            ("limit", "10"),
            ("page", "2"),
            ("sfw", "true"),
            ("unapproved", "1"),
            ("min_score", "7.5"),
            ("max_score", "9"),
            ("genres", "1,2"),
            ("genres_exclude", "9"),
            ("start_date", "2000-01-01"),
            ("end_date", "2010-12-31"),
            ("status", "airing"),
            ("type", "tv"),
            ("rating", "pg13"),
            ("producer", "1"),
            ("order_by", "start_date"),
        ]);
        let command = AnimeSearchCommand::parse(&query).unwrap();
        assert_eq!(command.q.as_deref(), Some("naruto"));
        assert_eq!(command.sort, Some(kuukan_core::enums::SortDirection::Asc));
        assert_eq!(command.limit, 10);
        assert_eq!(command.page, 2);
        assert!(command.sfw);
        assert!(command.unapproved);
        assert_eq!(command.min_score, Some(7.5));
        assert_eq!(command.max_score, Some(9.0));
        assert_eq!(command.start_date.unwrap().to_ymd(), "2000-01-01");
        assert_eq!(command.end_date.unwrap().to_ymd(), "2010-12-31");
        assert_eq!(command.status, Some(AnimeStatus::Airing));
        assert_eq!(command.r#type, Some(AnimeType::Tv));
        assert_eq!(command.rating, Some(AnimeRating::Pg13));
        assert_eq!(command.producer, Some(1));
        assert_eq!(command.order_by, Some(AnimeOrderBy::StartDate));
    }

    #[test]
    fn anime_search_limit_messages() {
        let query = Query::from_pairs([("limit", "abc")]);
        let messages = bag(AnimeSearchCommand::parse(&query).unwrap_err());
        assert_eq!(
            messages["limit"],
            serde_json::json!([
                "The limit must be a number.",
                "The limit must be an integer.",
                "Value abc is higher than the configured '25' max value."
            ])
        );
    }

    #[test]
    fn anime_search_page_message() {
        let query = Query::from_pairs([("page", "0")]);
        let messages = bag(AnimeSearchCommand::parse(&query).unwrap_err());
        assert_eq!(messages["page"], serde_json::json!(["The page must be at least 1."]));
    }

    #[test]
    fn anime_search_enum_messages() {
        let query = Query::from_pairs([
            ("status", "x"),
            ("type", "y"),
            ("rating", "z"),
            ("order_by", "w"),
        ]);
        let messages = bag(AnimeSearchCommand::parse(&query).unwrap_err());
        assert_eq!(
            messages["status"],
            serde_json::json!(["The status field is not a valid App\\Enums\\AnimeStatusEnum."])
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
            messages["order_by"],
            serde_json::json!(["The order by field is not a valid App\\Enums\\AnimeOrderByEnum."])
        );
    }

    #[test]
    fn anime_search_letter_and_producers_prohibitions() {
        let query = Query::from_pairs([("letter", "ab"), ("q", "naruto")]);
        let command = AnimeSearchCommand::parse(&query).unwrap_err();
        let messages = bag(command);
        assert_eq!(
            messages["letter"],
            serde_json::json!([
                "The letter must be 1 characters.",
                "The letter field prohibits q from being present."
            ])
        );

        let query = Query::from_pairs([("producers", "1,2"), ("producer", "1")]);
        let messages = bag(AnimeSearchCommand::parse(&query).unwrap_err());
        assert_eq!(
            messages["producers"],
            serde_json::json!(["The producers field prohibits producer from being present."])
        );
    }

    #[test]
    fn anime_search_score_messages() {
        let query = Query::from_pairs([("score", "0.5"), ("min_score", "1")]);
        let messages = bag(AnimeSearchCommand::parse(&query).unwrap_err());
        assert_eq!(
            messages["score"],
            serde_json::json!([
                "The score must be between 1 and 9.99.",
                "The score field prohibits min score / max score from being present."
            ])
        );

        let query =
            Query::from_pairs([("score", "0.5"), ("min_score", "1"), ("max_score", "9")]);
        let messages = bag(AnimeSearchCommand::parse(&query).unwrap_err());
        assert_eq!(
            messages["score"],
            serde_json::json!([
                "The score must be between 1 and 9.99.",
                "The score field prohibits min score / max score from being present."
            ])
        );

        let query = Query::from_pairs([("min_score", "8"), ("max_score", "5")]);
        let messages = bag(AnimeSearchCommand::parse(&query).unwrap_err());
        assert_eq!(
            messages["min_score"],
            serde_json::json!(["The min score must be less than or equal to 5."])
        );
        assert_eq!(
            messages["max_score"],
            serde_json::json!(["The max score must be greater than or equal to 8."])
        );
    }

    #[test]
    fn anime_search_date_messages() {
        let query = Query::from_pairs([("start_date", "garbage")]);
        let messages = bag(AnimeSearchCommand::parse(&query).unwrap_err());
        assert_eq!(
            messages["start_date"],
            serde_json::json!(["The start date does not match the format Y-m-d."])
        );

        let query = Query::from_pairs([
            ("start_date", "2021-01-01"),
            ("end_date", "2020-01-01"),
        ]);
        let messages = bag(AnimeSearchCommand::parse(&query).unwrap_err());
        assert_eq!(
            messages["start_date"],
            serde_json::json!([
                "The start date must be a date before or equal to end date."
            ])
        );
        assert_eq!(
            messages["end_date"],
            serde_json::json!([
                "The end date must be a date after or equal to start date."
            ])
        );
    }

    #[test]
    fn anime_search_q_control_characters() {
        let query = Query::from_pairs([("q", "naruto\n")]);
        let messages = bag(AnimeSearchCommand::parse(&query).unwrap_err());
        assert_eq!(
            messages["q"],
            serde_json::json!([
                "The q parameter cannot contain any of the following characters: \\n, \\r, \\t, \\0, %0A"
            ])
        );
    }

    #[test]
    fn anime_search_empty_strings_follow_prepares_data() {
        let query = Query::from_pairs([
            ("q", ""),
            ("limit", ""),
            ("page", ""),
            ("sfw", ""),
            ("unapproved", ""),
            ("letter", ""),
            ("min_score", ""),
        ]);
        let command = AnimeSearchCommand::parse(&query).unwrap();
        assert_eq!(command.q, None);
        assert_eq!(command.page, 1);
        // `limit` was dropped by PreparesData; the paginator default applies.
        assert_eq!(command.limit, 25);
        assert!(command.sfw);
        assert!(command.unapproved);
        assert_eq!(command.letter, None);
        assert_eq!(command.min_score, None);
    }

    #[test]
    fn anime_lookup_min_id() {
        let command = AnimeLookupCommand::parse(1, &Query::new()).unwrap();
        assert_eq!(command.id, 1);
        let messages = bag(AnimeLookupCommand::parse(0, &Query::new()).unwrap_err());
        assert_eq!(messages["id"], serde_json::json!(["The id must be at least 1."]));
    }

    #[test]
    fn anime_episode_lookup() {
        let command =
            AnimeEpisodeLookupCommand::parse(1, 5, &Query::new()).unwrap();
        assert_eq!(command.episode_id, 5);
        let messages =
            bag(AnimeEpisodeLookupCommand::parse(1, 0, &Query::new()).unwrap_err());
        assert_eq!(
            messages["episodeId"],
            serde_json::json!(["The episode id must be at least 1."])
        );
    }

    #[test]
    fn anime_episodes_page_validation() {
        let query = Query::from_pairs([("page", "x")]);
        let messages = bag(AnimeEpisodesLookupCommand::parse(1, &query).unwrap_err());
        assert_eq!(messages["page"], serde_json::json!(["The page must be a number."]));
    }

    #[test]
    fn anime_forum_filter_validation() {
        let query = Query::from_pairs([("filter", "x")]);
        let messages = bag(AnimeForumLookupCommand::parse(1, &query).unwrap_err());
        assert_eq!(
            messages["filter"],
            serde_json::json!(["The filter field is not a valid App\\Enums\\AnimeForumFilterEnum."])
        );
    }

    #[test]
    fn anime_reviews_lookup_validation() {
        let query = Query::from_pairs([("sort", "x")]);
        let messages = bag(AnimeReviewsLookupCommand::parse(1, &query).unwrap_err());
        assert_eq!(
            messages["sort"],
            serde_json::json!(["The sort field is not a valid App\\Enums\\MediaReviewsSortEnum."])
        );
    }

    #[test]
    fn anime_genre_list_filter_validation() {
        let query = Query::from_pairs([("filter", "bogus")]);
        let messages = bag(AnimeGenreListCommand::parse(&query).unwrap_err());
        assert_eq!(
            messages["filter"],
            serde_json::json!(["The filter field is not a valid App\\Enums\\GenreFilterEnum."])
        );
        assert!(AnimeGenreListCommand::parse(&Query::new()).is_ok());
    }

    #[test]
    fn anime_lookups_have_fingerprints() {
        let command = AnimeLookupCommand::parse(1, &Query::new()).unwrap();
        assert_eq!(
            command.request_fingerprint("/v4/anime/1"),
            "request:anime:1ab5e4f6f8cd1573c8ff865af201c125cdff1dcb"
        );
    }
}
