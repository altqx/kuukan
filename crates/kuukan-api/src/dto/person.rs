//! Person request commands — port of `app/Dto/Person*.php`.

use std::ops::Deref;

use kuukan_core::enums::PeopleOrderBy;
use kuukan_core::error::ApiError;
use kuukan_core::params::Query;

use super::base::{check_search_q, id_lookup_command, QueryParser, SearchCommand};

/// PHP `App\Dto\PeopleSearchCommand`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeopleSearchCommand {
    pub search: SearchCommand,
    pub order_by: Option<PeopleOrderBy>,
}

impl Deref for PeopleSearchCommand {
    type Target = SearchCommand;

    fn deref(&self) -> &Self::Target {
        &self.search
    }
}

impl PeopleSearchCommand {
    pub fn parse(query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::clean(query);
        let search = SearchCommand::parse_with(&mut parser);
        let order_by = parser.enum_optional::<PeopleOrderBy>("order_by", PeopleOrderBy::PHP_CLASS);
        parser.finish()?;
        let command = Self {
            search: search?,
            order_by: order_by?,
        };
        check_search_q(command.search.q.as_deref())?;
        Ok(command)
    }
}

id_lookup_command! {
    /// PHP `App\Dto\PersonLookupCommand`.
    PersonLookupCommand
}

id_lookup_command! {
    /// PHP `App\Dto\PersonFullLookupCommand`.
    PersonFullLookupCommand
}

id_lookup_command! {
    /// PHP `App\Dto\PersonAnimeLookupCommand`.
    PersonAnimeLookupCommand
}

id_lookup_command! {
    /// PHP `App\Dto\PersonMangaLookupCommand`.
    PersonMangaLookupCommand
}

id_lookup_command! {
    /// PHP `App\Dto\PersonVoicesLookupCommand`.
    PersonVoicesLookupCommand
}

id_lookup_command! {
    /// PHP `App\Dto\PersonPicturesLookupCommand`.
    PersonPicturesLookupCommand
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bag(err: ApiError) -> serde_json::Value {
        err.body(false)["messages"].clone()
    }

    #[test]
    fn people_search_defaults_and_order_by() {
        let command = PeopleSearchCommand::parse(&Query::new()).unwrap();
        assert_eq!(command.page, 1);
        assert_eq!(command.limit, 25);
        assert_eq!(command.order_by, None);

        let query = Query::from_pairs([("q", "miyazaki"), ("order_by", "birthday")]);
        let command = PeopleSearchCommand::parse(&query).unwrap();
        assert_eq!(command.q.as_deref(), Some("miyazaki"));
        assert_eq!(command.order_by, Some(PeopleOrderBy::Birthday));
    }

    #[test]
    fn people_search_invalid_order_by() {
        let query = Query::from_pairs([("order_by", "bogus")]);
        let messages = bag(PeopleSearchCommand::parse(&query).unwrap_err());
        assert_eq!(
            messages["order_by"],
            serde_json::json!(["The order by field is not a valid App\\Enums\\PeopleOrderByEnum."])
        );
    }

    #[test]
    fn person_lookup_min_id() {
        assert_eq!(PersonLookupCommand::parse(3, &Query::new()).unwrap().id, 3);
        let messages = bag(PersonLookupCommand::parse(0, &Query::new()).unwrap_err());
        assert_eq!(
            messages["id"],
            serde_json::json!(["The id must be at least 1."])
        );
    }
}
