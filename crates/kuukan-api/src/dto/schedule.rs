//! Weekly schedule command — port of `app/Dto/QueryAnimeSchedulesCommand.php`.

use kuukan_core::enums::AnimeScheduleFilter;
use kuukan_core::error::ApiError;
use kuukan_core::params::Query;

use super::base::{HasRequestFingerprint, QueryParser};

/// PHP `App\Dto\QueryAnimeSchedulesCommand` (route
/// `schedules[/{filter:[A-Za-z]+}]`; `filter` is nullable and has no
/// `PreparesData` empty-string cleanup beyond the trait the class uses).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryAnimeSchedulesCommand {
    pub sfw: bool,
    pub kids: Option<bool>,
    pub unapproved: bool,
    pub limit: u64,
    pub page: u64,
    pub filter: Option<AnimeScheduleFilter>,
}

impl HasRequestFingerprint for QueryAnimeSchedulesCommand {}

impl QueryAnimeSchedulesCommand {
    /// `filter` is the optional route parameter; `?filter=` is used when the
    /// route value is absent.
    pub fn parse(filter: Option<&str>, query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::clean(query);
        let sfw = parser.bool_flag("sfw", false);
        let kids = parser.optional_bool_flag("kids");
        let unapproved = parser.bool_flag("unapproved", false);
        let limit = parser.limit(None)?;
        let page = parser.page()?;
        let raw = filter.or_else(|| parser.get("filter"));
        let filter_value = parser.enum_nullable_raw::<AnimeScheduleFilter>(
            "filter",
            raw,
            AnimeScheduleFilter::PHP_CLASS,
        );
        parser.finish()?;
        Ok(Self {
            sfw,
            kids,
            unapproved,
            limit,
            page,
            filter: filter_value?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bag(err: ApiError) -> serde_json::Value {
        err.body(false).clone()
    }

    #[test]
    fn schedules_defaults() {
        let command = QueryAnimeSchedulesCommand::parse(None, &Query::new()).unwrap();
        assert_eq!(command.page, 1);
        assert_eq!(command.limit, 25);
        assert!(!command.sfw);
        assert_eq!(command.kids, None);
        assert_eq!(command.filter, None);
    }

    #[test]
    fn schedules_filter_values() {
        let command =
            QueryAnimeSchedulesCommand::parse(Some("monday"), &Query::new()).unwrap();
        assert_eq!(command.filter, Some(AnimeScheduleFilter::Monday));

        let query = Query::from_pairs([("filter", "sunday")]);
        let command = QueryAnimeSchedulesCommand::parse(None, &query).unwrap();
        assert_eq!(command.filter, Some(AnimeScheduleFilter::Sunday));
    }

    #[test]
    fn schedules_invalid_filter_and_bool() {
        let error =
            QueryAnimeSchedulesCommand::parse(Some("bogus"), &Query::new()).unwrap_err();
        let body = bag(error);
        assert_eq!(
            body["messages"]["filter"],
            serde_json::json!(["The filter field is not a valid App\\Enums\\AnimeScheduleFilterEnum."])
        );

        let query = Query::from_pairs([("kids", "maybe")]);
        let body = bag(
            QueryAnimeSchedulesCommand::parse(None, &query).unwrap_err(),
        );
        assert_eq!(
            body["messages"]["kids"],
            serde_json::json!(["The kids field must be true or false."])
        );
    }

    #[test]
    fn schedules_empty_filter_is_a_cast_error() {
        // `filter` is nullable (not Optional): the empty string skips the
        // `EnumRule` and then `EnumCast` throws, exactly like PHP.
        let error = QueryAnimeSchedulesCommand::parse(Some(""), &Query::new())
            .unwrap_err();
        assert_eq!(error.status(), 500);
    }
}
