//! Shared search command bases — port of `app/Dto/SearchCommand.php` and
//! `app/Dto/MediaSearchCommand.php`.
//!
//! The concrete command structs re-exported here live in [`crate::dto::base`]:
//!
//! * [`SearchCommand`] — `q`, `sort`, `letter`, `page`, `limit`;
//! * [`MediaSearchCommand`] — `SearchCommand` plus `sfw`, `unapproved`,
//!   `min_score`, `max_score`, `score`, `genres`, `genres_exclude` and the
//!   `start_date`/`end_date` range.

pub use super::base::{check_search_q, MediaSearchCommand, SearchCommand};

#[cfg(test)]
mod tests {
    use super::*;
    use kuukan_core::params::Query;

    #[test]
    fn search_command_defaults() {
        let command = SearchCommand::parse(&Query::new()).unwrap();
        assert_eq!(command.page, 1);
        assert_eq!(command.limit, 25);
        assert_eq!(command.q, None);
        assert_eq!(command.sort, None);
        assert_eq!(command.letter, None);
    }

    #[test]
    fn media_search_defaults() {
        let command = MediaSearchCommand::parse(&Query::new()).unwrap();
        assert!(!command.sfw);
        assert!(!command.unapproved);
        assert_eq!(command.start_date, None);
        assert_eq!(command.end_date, None);
    }
}
