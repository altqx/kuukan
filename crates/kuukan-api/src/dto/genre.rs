//! Genre request commands — port of `app/Dto/GenreListCommand.php`.
//!
//! `GenreListCommand` is the abstract PHP base; the concrete
//! `AnimeGenreListCommand` / `MangaGenreListCommand` live in
//! [`crate::dto::anime`] and [`crate::dto::manga`].

pub use super::base::GenreListCommand;

#[cfg(test)]
mod tests {
    use super::*;
    use kuukan_core::enums::GenreFilter;
    use kuukan_core::params::Query;

    #[test]
    fn genre_list_filter() {
        let command = GenreListCommand::parse(&Query::new()).unwrap();
        assert_eq!(command.filter, None);

        let query = Query::from_pairs([("filter", "explicit_genres")]);
        let command = GenreListCommand::parse(&query).unwrap();
        assert_eq!(command.filter, Some(GenreFilter::ExplicitGenres));

        let query = Query::from_pairs([("filter", "bogus")]);
        let err = GenreListCommand::parse(&query).unwrap_err();
        assert_eq!(
            err.body(false)["messages"]["filter"][0],
            "The filter field is not a valid App\\Enums\\GenreFilterEnum."
        );
    }
}
