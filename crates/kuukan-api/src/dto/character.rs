//! Character request commands — port of `app/Dto/Character*.php`.

use std::ops::Deref;

use kuukan_core::enums::CharacterOrderBy;
use kuukan_core::error::ApiError;
use kuukan_core::params::Query;

use super::base::{check_search_q, id_lookup_command, QueryParser, SearchCommand};

/// PHP `App\Dto\CharactersSearchCommand`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharactersSearchCommand {
    pub search: SearchCommand,
    pub order_by: Option<CharacterOrderBy>,
}

impl Deref for CharactersSearchCommand {
    type Target = SearchCommand;

    fn deref(&self) -> &Self::Target {
        &self.search
    }
}

impl CharactersSearchCommand {
    pub fn parse(query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::clean(query);
        let search = SearchCommand::parse_with(&mut parser);
        let order_by =
            parser.enum_optional::<CharacterOrderBy>("order_by", CharacterOrderBy::PHP_CLASS);
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
    /// PHP `App\Dto\CharacterLookupCommand`.
    CharacterLookupCommand
}

id_lookup_command! {
    /// PHP `App\Dto\CharacterFullLookupCommand`.
    CharacterFullLookupCommand
}

id_lookup_command! {
    /// PHP `App\Dto\CharacterAnimeLookupCommand`.
    CharacterAnimeLookupCommand
}

id_lookup_command! {
    /// PHP `App\Dto\CharacterMangaLookupCommand`.
    CharacterMangaLookupCommand
}

id_lookup_command! {
    /// PHP `App\Dto\CharacterVoicesLookupCommand`.
    CharacterVoicesLookupCommand
}

id_lookup_command! {
    /// PHP `App\Dto\CharacterPicturesLookupCommand`.
    CharacterPicturesLookupCommand
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bag(err: ApiError) -> serde_json::Value {
        err.body(false)["messages"].clone()
    }

    use kuukan_core::enums::SortDirection;

    #[test]
    fn characters_search_defaults_and_order_by() {
        let command = CharactersSearchCommand::parse(&Query::new()).unwrap();
        assert_eq!(command.page, 1);
        assert_eq!(command.limit, 25);
        assert_eq!(command.order_by, None);

        let query = Query::from_pairs([("q", "edward"), ("sort", "desc")]);
        let command = CharactersSearchCommand::parse(&query).unwrap();
        assert_eq!(command.q.as_deref(), Some("edward"));
        assert_eq!(command.sort, Some(SortDirection::Desc));
    }

    #[test]
    fn characters_search_invalid_order_by() {
        let query = Query::from_pairs([("order_by", "bogus")]);
        let messages = bag(CharactersSearchCommand::parse(&query).unwrap_err());
        assert_eq!(
            messages["order_by"],
            serde_json::json!([
                "The order by field is not a valid App\\Enums\\CharacterOrderByEnum."
            ])
        );
    }

    #[test]
    fn character_lookup_min_id() {
        assert_eq!(
            CharacterLookupCommand::parse(1, &Query::new()).unwrap().id,
            1
        );
        let messages = bag(CharacterLookupCommand::parse(0, &Query::new()).unwrap_err());
        assert_eq!(
            messages["id"],
            serde_json::json!(["The id must be at least 1."])
        );
    }
}
