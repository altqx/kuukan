//! Club request commands — port of `app/Dto/Club*.php`.

use std::ops::Deref;

use kuukan_core::enums::{ClubCategory, ClubOrderBy, ClubType};
use kuukan_core::error::ApiError;
use kuukan_core::params::Query;

use super::base::{
    check_search_q, id_lookup_command, id_page_lookup_command,     QueryParser, SearchCommand,
};

/// PHP `App\Dto\ClubSearchCommand`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClubSearchCommand {
    pub search: SearchCommand,
    pub category: Option<ClubCategory>,
    pub r#type: Option<ClubType>,
    pub order_by: Option<ClubOrderBy>,
}

impl Deref for ClubSearchCommand {
    type Target = SearchCommand;

    fn deref(&self) -> &Self::Target {
        &self.search
    }
}

impl ClubSearchCommand {
    pub fn parse(query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::clean(query);
        let search = SearchCommand::parse_with(&mut parser);
        let category =
            parser.enum_optional::<ClubCategory>("category", ClubCategory::PHP_CLASS);
        let club_type = parser.enum_optional::<ClubType>("type", ClubType::PHP_CLASS);
        let order_by =
            parser.enum_optional::<ClubOrderBy>("order_by", ClubOrderBy::PHP_CLASS);
        parser.finish()?;
        let command = Self {
            search: search?,
            category: category?,
            r#type: club_type?,
            order_by: order_by?,
        };
        check_search_q(command.search.q.as_deref())?;
        Ok(command)
    }
}

id_lookup_command! {
    /// PHP `App\Dto\ClubLookupCommand`.
    ClubLookupCommand
}

id_page_lookup_command! {
    /// PHP `App\Dto\ClubMembersLookupCommand`.
    ClubMembersLookupCommand
}

id_lookup_command! {
    /// PHP `App\Dto\ClubStaffLookupCommand`.
    ClubStaffLookupCommand
}

id_lookup_command! {
    /// PHP `App\Dto\ClubRelationLookupCommand`.
    ClubRelationLookupCommand
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bag(err: ApiError) -> serde_json::Value {
        err.body(false)["messages"].clone()
    }

    #[test]
    fn club_search_defaults_and_values() {
        let command = ClubSearchCommand::parse(&Query::new()).unwrap();
        assert_eq!(command.page, 1);
        assert_eq!(command.limit, 25);
        assert_eq!(command.category, None);
        assert_eq!(command.r#type, None);

        let query = Query::from_pairs([
            ("category", "anime"),
            ("type", "public"),
            ("order_by", "members_count"),
        ]);
        let command = ClubSearchCommand::parse(&query).unwrap();
        assert_eq!(command.category, Some(ClubCategory::Anime));
        assert_eq!(command.r#type, Some(ClubType::Public));
        assert_eq!(command.order_by, Some(ClubOrderBy::MembersCount));
    }

    #[test]
    fn club_search_invalid_messages() {
        let query = Query::from_pairs([("category", "x"), ("type", "y"), ("order_by", "z")]);
        let messages = bag(ClubSearchCommand::parse(&query).unwrap_err());
        assert_eq!(
            messages["category"],
            serde_json::json!(["The category field is not a valid App\\Enums\\ClubCategoryEnum."])
        );
        assert_eq!(
            messages["type"],
            serde_json::json!(["The type field is not a valid App\\Enums\\ClubTypeEnum."])
        );
        assert_eq!(
            messages["order_by"],
            serde_json::json!(["The order by field is not a valid App\\Enums\\ClubOrderByEnum."])
        );
    }

    #[test]
    fn club_members_page() {
        let command = ClubMembersLookupCommand::parse(1, &Query::from_pairs([("page", "3")])).unwrap();
        assert_eq!(command.page, 3);
        let messages =
            bag(ClubMembersLookupCommand::parse(1, &Query::from_pairs([("page", "x")])).unwrap_err());
        assert_eq!(messages["page"], serde_json::json!(["The page must be a number."]));
    }
}
