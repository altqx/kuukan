//! HTML/JSON parsers, ported from jikan-php.
//!
//! Parser methods are `pub(crate)` or private: the only thing outside this
//! module ever calls is `ParseModel::model`, and a getter that exists to build
//! a model is implementation, not interface.
//!
//! Some ported getters are deliberately unreachable — `MangaReviewScoresParser`
//! says so in its own doc comment — because they mirror a PHP method that the
//! upstream model never calls either. They are kept for parity with the
//! jikan-php namespace, so `dead_code` is allowed for the whole module rather
//! than annotated at fifty separate sites.
#![allow(dead_code)]

pub mod anime;
pub mod character;
pub mod club;
pub mod common;
pub mod date;
pub mod forum;
pub mod genre;
pub mod helper;
pub mod jstring;
pub mod magazine;
pub mod mal_url;
pub mod manga;
pub mod media_url;
pub mod news;
pub mod person;
pub mod producer;
pub mod recommendations;
pub mod reviews;
pub mod schedule;
pub mod search;
pub mod season_list;
pub mod seasonal;
pub mod top;
pub mod user;
pub mod watch;

use serde_json::Value;

use crate::error::ParseError;
use crate::parser::helper::HtmlDoc;

/// One entry point for every parser that yields a model document.
///
/// The parser types spelled the same operation nine different ways —
/// `get_model`, `model`, `characters`, `get_results`, `more_info` and so on —
/// so which name to call was a fact every caller had to carry. That is
/// interface, and it varied for no reason. Each parser's own spelling survives
/// as an implementation detail behind this one.
///
/// Eight parsers are deliberately absent: they yield an `Option<String>`, a
/// bare list, or a different error type, and pretending otherwise would only
/// move the special case somewhere less visible.
pub trait ParseModel {
    /// Turn a fetched document into its API-shaped model.
    fn model(doc: HtmlDoc) -> Result<Value, ParseError>;
}

impl ParseModel for crate::parser::anime::AnimeParser {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::anime::AnimeParser::new(doc).get_model()
    }
}

impl ParseModel for crate::parser::anime::AnimeRecentlyUpdatedByUsersParser {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::anime::AnimeRecentlyUpdatedByUsersParser::new(doc).get_model()
    }
}

impl ParseModel for crate::parser::anime::AnimeReviewsParser {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::anime::AnimeReviewsParser::new(doc).get_model()
    }
}

impl ParseModel for crate::parser::anime::AnimeStatsParser {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::anime::AnimeStatsParser::new(doc).get_model()
    }
}

impl ParseModel for crate::parser::anime::CharactersAndStaffParser {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::anime::CharactersAndStaffParser::new(doc).get_model()
    }
}

impl ParseModel for crate::parser::anime::EpisodesParser {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::anime::EpisodesParser::new(doc).get_model()
    }
}

impl ParseModel for crate::parser::anime::VideosParser {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::anime::VideosParser::new(doc).get_results_model()
    }
}

impl ParseModel for crate::parser::character::CharacterParser {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::character::CharacterParser::new(doc).model()
    }
}

impl ParseModel for crate::parser::club::ClubParser {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::club::ClubParser::new(doc).model()
    }
}

impl ParseModel for crate::parser::club::UserListParser {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::club::UserListParser::new(doc).model()
    }
}

impl ParseModel for crate::parser::genre::AnimeGenreListParser {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::genre::AnimeGenreListParser::new(doc).model()
    }
}

impl ParseModel for crate::parser::genre::AnimeGenreParser {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::genre::AnimeGenreParser::new(doc).model()
    }
}

impl ParseModel for crate::parser::genre::MangaGenreListParser {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::genre::MangaGenreListParser::new(doc).model()
    }
}

impl ParseModel for crate::parser::genre::MangaGenreParser {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::genre::MangaGenreParser::new(doc).model()
    }
}

impl ParseModel for crate::parser::magazine::MagazineListParser {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::magazine::MagazineListParser::new(doc).model()
    }
}

impl ParseModel for crate::parser::magazine::MagazineParser {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::magazine::MagazineParser::new(doc).model()
    }
}

impl ParseModel for crate::parser::manga::MangaParser {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::manga::MangaParser::new(doc).model()
    }
}

impl ParseModel for crate::parser::manga::MangaRecentlyUpdatedByUsersParser {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::manga::MangaRecentlyUpdatedByUsersParser::new(doc).model()
    }
}

impl ParseModel for crate::parser::manga::MangaReviewsParser {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::manga::MangaReviewsParser::new(doc).model()
    }
}

impl ParseModel for crate::parser::manga::MangaStatsParser {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::manga::MangaStatsParser::new(doc).model()
    }
}

impl ParseModel for crate::parser::person::PersonParser {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::person::PersonParser::new(doc).model()
    }
}

impl ParseModel for crate::parser::producer::ProducerListParser {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::producer::ProducerListParser::new(doc).model()
    }
}

impl ParseModel for crate::parser::producer::ProducerParser {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::producer::ProducerParser::new(doc).model()
    }
}

impl ParseModel for crate::parser::recommendations::RecentRecommendationsParser<'_> {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::recommendations::RecentRecommendationsParser::new(&doc).get_model()
    }
}

impl ParseModel for crate::parser::reviews::ReviewsParser<'_> {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::reviews::ReviewsParser::new(&doc).get_model()
    }
}

impl ParseModel for crate::parser::schedule::ScheduleParser<'_> {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::schedule::ScheduleParser::new(&doc).get_model()
    }
}

impl ParseModel for crate::parser::search::AnimeSearchParser<'_> {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::search::AnimeSearchParser::new(&doc).get_model()
    }
}

impl ParseModel for crate::parser::search::CharacterSearchParser<'_> {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::search::CharacterSearchParser::new(&doc).get_model()
    }
}

impl ParseModel for crate::parser::search::MangaSearchParser<'_> {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::search::MangaSearchParser::new(&doc).get_model()
    }
}

impl ParseModel for crate::parser::search::PersonSearchParser<'_> {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::search::PersonSearchParser::new(&doc).get_model()
    }
}

impl ParseModel for crate::parser::season_list::SeasonListParser<'_> {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::season_list::SeasonListParser::new(&doc).get_model()
    }
}

impl ParseModel for crate::parser::seasonal::SeasonalParser<'_> {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::seasonal::SeasonalParser::new(&doc).get_model()
    }
}

impl ParseModel for crate::parser::top::TopAnimeParser<'_> {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::top::TopAnimeParser::new(&doc).get_model()
    }
}

impl ParseModel for crate::parser::top::TopCharactersParser<'_> {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::top::TopCharactersParser::new(&doc).get_model()
    }
}

impl ParseModel for crate::parser::top::TopMangaParser<'_> {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::top::TopMangaParser::new(&doc).get_model()
    }
}

impl ParseModel for crate::parser::top::TopPeopleParser<'_> {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::top::TopPeopleParser::new(&doc).get_model()
    }
}

impl ParseModel for crate::parser::user::FriendsParser {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::user::FriendsParser::new(&doc).get_model()
    }
}

impl ParseModel for crate::parser::user::UserProfileParser {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::user::UserProfileParser::new(&doc).get_model()
    }
}

impl ParseModel for crate::parser::user::UserRecommendationsParser {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::user::UserRecommendationsParser::new(&doc).get_model()
    }
}

impl ParseModel for crate::parser::user::UserReviewsParser {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::user::UserReviewsParser::new(&doc).get_model()
    }
}

impl ParseModel for crate::parser::user::UsernameByIdParser {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::user::UsernameByIdParser::new(&doc).get_user()
    }
}

impl ParseModel for crate::parser::watch::WatchEpisodesParser<'_> {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::watch::WatchEpisodesParser::new(&doc).get_model()
    }
}

impl ParseModel for crate::parser::watch::WatchPromotionalVideosParser<'_> {
    fn model(doc: HtmlDoc) -> Result<Value, ParseError> {
        crate::parser::watch::WatchPromotionalVideosParser::new(&doc).get_model()
    }
}
