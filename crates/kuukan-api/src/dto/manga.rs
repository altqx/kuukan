//! Manga request commands — port of `app/Dto/Manga*.php`.

use std::ops::Deref;

use kuukan_core::enums::{
    MangaForumFilter, MangaOrderBy, MangaStatus, MangaType,
};
use kuukan_core::error::ApiError;
use kuukan_core::params::Query;

use super::base::{
    check_search_q, id_lookup_command, id_page_lookup_command, GenreListCommand,
    HasRequestFingerprint, MediaSearchCommand, QueryParser, QueryReviewsCommand,
};

/// PHP `App\Dto\MangaSearchCommand`.
#[derive(Debug, Clone, PartialEq)]
pub struct MangaSearchCommand {
    pub media: MediaSearchCommand,
    pub status: Option<MangaStatus>,
    pub r#type: Option<MangaType>,
    pub magazines: Option<String>,
    pub order_by: Option<MangaOrderBy>,
}

impl Deref for MangaSearchCommand {
    type Target = MediaSearchCommand;

    fn deref(&self) -> &Self::Target {
        &self.media
    }
}

impl MangaSearchCommand {
    pub fn parse(query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::clean(query);
        let media = MediaSearchCommand::parse_with(&mut parser);
        let status =
            parser.enum_optional::<MangaStatus>("status", MangaStatus::PHP_CLASS);
        let manga_type = parser.enum_optional::<MangaType>("type", MangaType::PHP_CLASS);
        let magazines = parser.optional_string("magazines");
        let order_by = parser.enum_optional::<MangaOrderBy>(
            "order_by",
            MangaOrderBy::PHP_CLASS,
        );
        parser.finish()?;
        let command = Self {
            media: media?,
            status: status?,
            r#type: manga_type?,
            magazines,
            order_by: order_by?,
        };
        check_search_q(command.media.search.q.as_deref())?;
        Ok(command)
    }
}

id_lookup_command! {
    /// PHP `App\Dto\MangaLookupCommand`.
    MangaLookupCommand
}

id_lookup_command! {
    /// PHP `App\Dto\MangaFullLookupCommand`.
    MangaFullLookupCommand
}

id_lookup_command! {
    /// PHP `App\Dto\MangaCharactersLookupCommand`.
    MangaCharactersLookupCommand
}

id_lookup_command! {
    /// PHP `App\Dto\MangaExternalLookupCommand`.
    MangaExternalLookupCommand
}

/// PHP `App\Dto\MangaForumLookupCommand` (`filter` query parameter).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MangaForumLookupCommand {
    pub id: i64,
    pub filter: Option<MangaForumFilter>,
}

impl HasRequestFingerprint for MangaForumLookupCommand {}

impl MangaForumLookupCommand {
    pub fn parse(id: i64, query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::new(query);
        let id = parser.id(id);
        let filter = parser.enum_optional::<MangaForumFilter>(
            "filter",
            MangaForumFilter::PHP_CLASS,
        );
        parser.finish()?;
        Ok(Self { id, filter: filter? })
    }
}

id_lookup_command! {
    /// PHP `App\Dto\MangaMoreInfoLookupCommand`.
    MangaMoreInfoLookupCommand
}

id_page_lookup_command! {
    /// PHP `App\Dto\MangaNewsLookupCommand`.
    MangaNewsLookupCommand
}

id_lookup_command! {
    /// PHP `App\Dto\MangaPicturesLookupCommand`.
    MangaPicturesLookupCommand
}

id_lookup_command! {
    /// PHP `App\Dto\MangaRecommendationsLookupCommand`.
    MangaRecommendationsLookupCommand
}

id_lookup_command! {
    /// PHP `App\Dto\MangaRelationsLookupCommand`.
    MangaRelationsLookupCommand
}

/// PHP `App\Dto\MangaReviewsLookupCommand` (carries `PreparesData`).
#[derive(Debug, Clone, PartialEq)]
pub struct MangaReviewsLookupCommand {
    pub id: i64,
    pub reviews: QueryReviewsCommand,
}

impl Deref for MangaReviewsLookupCommand {
    type Target = QueryReviewsCommand;

    fn deref(&self) -> &Self::Target {
        &self.reviews
    }
}

impl HasRequestFingerprint for MangaReviewsLookupCommand {}

impl MangaReviewsLookupCommand {
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
    /// PHP `App\Dto\MangaStatsLookupCommand`.
    MangaStatsLookupCommand
}

id_page_lookup_command! {
    /// PHP `App\Dto\MangaUserUpdatesLookupCommand`.
    MangaUserUpdatesLookupCommand
}

/// PHP `App\Dto\MangaGenreListCommand` (`filter` query parameter).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MangaGenreListCommand {
    pub genres: GenreListCommand,
}

impl Deref for MangaGenreListCommand {
    type Target = GenreListCommand;

    fn deref(&self) -> &Self::Target {
        &self.genres
    }
}

impl MangaGenreListCommand {
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
    fn manga_search_defaults_and_values() {
        let command = MangaSearchCommand::parse(&Query::new()).unwrap();
        assert_eq!(command.page, 1);
        assert_eq!(command.limit, 25);
        assert!(!command.sfw);
        assert_eq!(command.status, None);
        assert_eq!(command.magazines, None);

        let query = Query::from_pairs([
            ("q", "berserk"),
            ("type", "manga"),
            ("status", "publishing"),
            ("magazines", "1,2"),
            ("order_by", "start_date"),
        ]);
        let command = MangaSearchCommand::parse(&query).unwrap();
        assert_eq!(command.q.as_deref(), Some("berserk"));
        assert_eq!(command.r#type, Some(MangaType::Manga));
        assert_eq!(command.status, Some(MangaStatus::Publishing));
        assert_eq!(command.order_by, Some(MangaOrderBy::StartDate));
    }

    #[test]
    fn manga_search_invalid_messages() {
        let query = Query::from_pairs([("type", "x"), ("status", "y"), ("order_by", "z")]);
        let messages = bag(MangaSearchCommand::parse(&query).unwrap_err());
        assert_eq!(
            messages["type"],
            serde_json::json!(["The type field is not a valid App\\Enums\\MangaTypeEnum."])
        );
        assert_eq!(
            messages["status"],
            serde_json::json!(["The status field is not a valid App\\Enums\\MangaStatusEnum."])
        );
        assert_eq!(
            messages["order_by"],
            serde_json::json!(["The order by field is not a valid App\\Enums\\MangaOrderByEnum."])
        );
    }

    #[test]
    fn manga_lookup_and_page() {
        assert_eq!(MangaLookupCommand::parse(7, &Query::new()).unwrap().id, 7);
        let query = Query::from_pairs([("page", "2")]);
        let command = MangaNewsLookupCommand::parse(1, &query).unwrap();
        assert_eq!(command.page, 2);
        let messages = bag(MangaNewsLookupCommand::parse(1, &Query::from_pairs([("page", "x")])).unwrap_err());
        assert_eq!(messages["page"], serde_json::json!(["The page must be a number."]));
    }

    #[test]
    fn manga_forum_filter() {
        let query = Query::from_pairs([("filter", "chapters")]);
        let command = MangaForumLookupCommand::parse(1, &query).unwrap();
        assert_eq!(command.filter, Some(MangaForumFilter::Chapters));
        let messages = bag(
            MangaForumLookupCommand::parse(1, &Query::from_pairs([("filter", "x")]))
                .unwrap_err(),
        );
        assert_eq!(
            messages["filter"],
            serde_json::json!(["The filter field is not a valid App\\Enums\\MangaForumFilterEnum."])
        );
    }

    #[test]
    fn manga_reviews_and_genres() {
        let query = Query::from_pairs([("sort", "newest"), ("preliminary", "true")]);
        let command = MangaReviewsLookupCommand::parse(1, &query).unwrap();
        assert_eq!(command.page, 1);
        assert_eq!(
            command.reviews.sort,
            Some(kuukan_core::enums::MediaReviewsSort::Newest)
        );
        assert_eq!(command.reviews.preliminary, Some(true));

        assert!(MangaGenreListCommand::parse(&Query::from_pairs([("filter", "themes")])).is_ok());
    }
}
