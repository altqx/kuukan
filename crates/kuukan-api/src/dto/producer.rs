//! Producer request commands — port of `app/Dto/Producer*.php`.

use std::ops::Deref;

use kuukan_core::enums::ProducerOrderBy;
use kuukan_core::error::ApiError;
use kuukan_core::params::Query;

use super::base::{check_search_q, id_lookup_command, QueryParser, SearchCommand};

/// PHP `App\Dto\ProducersSearchCommand`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProducersSearchCommand {
    pub search: SearchCommand,
    pub order_by: Option<ProducerOrderBy>,
}

impl Deref for ProducersSearchCommand {
    type Target = SearchCommand;

    fn deref(&self) -> &Self::Target {
        &self.search
    }
}

impl ProducersSearchCommand {
    pub fn parse(query: &Query) -> Result<Self, ApiError> {
        let mut parser = QueryParser::clean(query);
        let search = SearchCommand::parse_with(&mut parser);
        let order_by =
            parser.enum_optional::<ProducerOrderBy>("order_by", ProducerOrderBy::PHP_CLASS);
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
    /// PHP `App\Dto\ProducerLookupCommand`.
    ProducerLookupCommand
}

id_lookup_command! {
    /// PHP `App\Dto\ProducerFullLookupCommand`.
    ProducerFullLookupCommand
}

id_lookup_command! {
    /// PHP `App\Dto\ProducerExternalLookupCommand`.
    ProducerExternalLookupCommand
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bag(err: ApiError) -> serde_json::Value {
        err.body(false)["messages"].clone()
    }

    #[test]
    fn producers_search_defaults_and_order_by() {
        let command = ProducersSearchCommand::parse(&Query::new()).unwrap();
        assert_eq!(command.page, 1);
        assert_eq!(command.limit, 25);
        assert_eq!(command.order_by, None);

        let query = Query::from_pairs([("q", "ufotable"), ("order_by", "established")]);
        let command = ProducersSearchCommand::parse(&query).unwrap();
        assert_eq!(command.order_by, Some(ProducerOrderBy::Established));
    }

    #[test]
    fn producers_search_invalid_order_by() {
        let query = Query::from_pairs([("order_by", "bogus")]);
        let messages = bag(ProducersSearchCommand::parse(&query).unwrap_err());
        assert_eq!(
            messages["order_by"],
            serde_json::json!([
                "The order by field is not a valid App\\Enums\\ProducerOrderByEnum."
            ])
        );
    }

    #[test]
    fn producer_lookup_min_id() {
        assert_eq!(
            ProducerLookupCommand::parse(1, &Query::new()).unwrap().id,
            1
        );
        let messages = bag(ProducerLookupCommand::parse(0, &Query::new()).unwrap_err());
        assert_eq!(
            messages["id"],
            serde_json::json!(["The id must be at least 1."])
        );
    }
}
