//! Magazine request commands — port of `app/Dto/MagazineSearchCommand.php`.

use std::ops::Deref;

use kuukan_core::enums::MagazineOrderBy;
use kuukan_core::error::ApiError;
use kuukan_core::params::Query;

use super::base::{check_search_q, QueryParser, SearchCommand};

/// PHP `App\Dto\MagazineSearchCommand`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MagazineSearchCommand {
    pub search: SearchCommand,
    pub order_by: Option<MagazineOrderBy>,
}

impl Deref for MagazineSearchCommand {
    type Target = SearchCommand;

    fn deref(&self) -> &Self::Target {
        &self.search
    }
}

impl MagazineSearchCommand {
    pub fn parse(query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::clean(query);
        let search = SearchCommand::parse_with(&mut parser);
        let order_by =
            parser.enum_optional::<MagazineOrderBy>("order_by", MagazineOrderBy::PHP_CLASS);
        parser.finish()?;
        let command = Self {
            search: search?,
            order_by: order_by?,
        };
        check_search_q(command.search.q.as_deref())?;
        Ok(command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bag(err: ApiError) -> serde_json::Value {
        err.body(false)["messages"].clone()
    }

    #[test]
    fn magazine_search_defaults_and_values() {
        let command = MagazineSearchCommand::parse(&Query::new()).unwrap();
        assert_eq!(command.page, 1);
        assert_eq!(command.limit, 25);
        assert_eq!(command.order_by, None);

        let query = Query::from_pairs([("q", "jump"), ("order_by", "count")]);
        let command = MagazineSearchCommand::parse(&query).unwrap();
        assert_eq!(command.order_by, Some(MagazineOrderBy::Count));
    }

    #[test]
    fn magazine_search_invalid_order_by() {
        let query = Query::from_pairs([("order_by", "bogus")]);
        let messages = bag(MagazineSearchCommand::parse(&query).unwrap_err());
        assert_eq!(
            messages["order_by"],
            serde_json::json!([
                "The order by field is not a valid App\\Enums\\MagazineOrderByEnum."
            ])
        );
    }
}
