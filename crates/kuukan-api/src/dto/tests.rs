//! Cross-family parity tests.
//!
//! The expected message bags below were captured from the reference PHP
//! implementation (`jikan-rest` with `spatie/laravel-data` 3.11 and
//! `lumen-framework` 9.1.5) by running each DTO through
//! `prepareForPipeline()` + `validate()`, so they pin the exact Lumen
//! `validation.php` wording and rule order.

use kuukan_core::error::ApiError;
use kuukan_core::params::Query;

use crate::dto::anime::*;
use crate::dto::base::{GenreListCommand, HasRequestFingerprint};
use crate::dto::character::*;
use crate::dto::club::*;
use crate::dto::magazine::*;
use crate::dto::manga::*;
use crate::dto::misc::*;
use crate::dto::person::*;
use crate::dto::producer::*;
use crate::dto::recommendations::*;
use crate::dto::reviews::*;
use crate::dto::schedule::*;
use crate::dto::seasonal::*;
use crate::dto::top::*;
use crate::dto::user::*;
use crate::dto::watch::*;

fn bag(err: ApiError) -> serde_json::Value {
    err.body(false)["messages"].clone()
}

#[test]
fn every_command_parses_defaults() {
    // Anime
    assert!(AnimeSearchCommand::parse(&Query::new()).is_ok());
    for id in [1_i64] {
        assert!(AnimeLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(AnimeFullLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(AnimeCharactersLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(AnimeStaffLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(AnimeEpisodesLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(AnimeNewsLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(AnimeForumLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(AnimeVideosLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(AnimeVideosEpisodesLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(AnimePicturesLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(AnimeStatsLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(AnimeMoreInfoLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(AnimeRecommendationsLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(AnimeUserUpdatesLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(AnimeReviewsLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(AnimeRelationsLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(AnimeThemesLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(AnimeExternalLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(AnimeStreamingLookupCommand::parse(id, &Query::new()).is_ok());

        assert!(MangaLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(MangaFullLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(MangaCharactersLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(MangaExternalLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(MangaForumLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(MangaMoreInfoLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(MangaNewsLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(MangaPicturesLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(MangaRecommendationsLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(MangaRelationsLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(MangaReviewsLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(MangaStatsLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(MangaUserUpdatesLookupCommand::parse(id, &Query::new()).is_ok());

        assert!(CharacterLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(CharacterFullLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(CharacterAnimeLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(CharacterMangaLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(CharacterVoicesLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(CharacterPicturesLookupCommand::parse(id, &Query::new()).is_ok());

        assert!(PersonLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(PersonFullLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(PersonAnimeLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(PersonMangaLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(PersonVoicesLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(PersonPicturesLookupCommand::parse(id, &Query::new()).is_ok());

        assert!(ProducerLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(ProducerFullLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(ProducerExternalLookupCommand::parse(id, &Query::new()).is_ok());

        assert!(ClubLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(ClubMembersLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(ClubStaffLookupCommand::parse(id, &Query::new()).is_ok());
        assert!(ClubRelationLookupCommand::parse(id, &Query::new()).is_ok());

        assert!(UserByIdLookupCommand::parse(id, &Query::new()).is_ok());
    }
    assert!(AnimeEpisodeLookupCommand::parse(1, 5, &Query::new()).is_ok());

    // Searches
    assert!(MangaSearchCommand::parse(&Query::new()).is_ok());
    assert!(CharactersSearchCommand::parse(&Query::new()).is_ok());
    assert!(PeopleSearchCommand::parse(&Query::new()).is_ok());
    assert!(ProducersSearchCommand::parse(&Query::new()).is_ok());
    assert!(MagazineSearchCommand::parse(&Query::new()).is_ok());
    assert!(ClubSearchCommand::parse(&Query::new()).is_ok());
    assert!(UsersSearchCommand::parse(&Query::new()).is_ok());

    // Genres
    assert!(GenreListCommand::parse(&Query::new()).is_ok());
    assert!(AnimeGenreListCommand::parse(&Query::new()).is_ok());
    assert!(MangaGenreListCommand::parse(&Query::new()).is_ok());

    // Users
    for username in ["nekomata"] {
        assert!(UserAboutLookupCommand::parse(username, &Query::new()).is_ok());
        assert!(UserFullLookupCommand::parse(username, &Query::new()).is_ok());
        assert!(UserProfileLookupCommand::parse(username, &Query::new()).is_ok());
        assert!(UserStatisticsLookupCommand::parse(username, &Query::new()).is_ok());
        assert!(UserFavoritesLookupCommand::parse(username, &Query::new()).is_ok());
        assert!(UserExternalLookupCommand::parse(username, &Query::new()).is_ok());
        assert!(UserUpdatesLookupCommand::parse(username, &Query::new()).is_ok());
        assert!(UserClubsLookupCommand::parse(username, &Query::new()).is_ok());
        assert!(UserFriendsLookupCommand::parse(username, &Query::new()).is_ok());
        assert!(UserReviewsLookupCommand::parse(username, &Query::new()).is_ok());
        assert!(UserRecommendationsLookupCommand::parse(username, &Query::new()).is_ok());
        assert!(UserHistoryLookupCommand::parse(username, &Query::new()).is_ok());
        assert!(QueryAnimeListOfUserCommand::parse(username, &Query::new()).is_ok());
        assert!(QueryMangaListOfUserCommand::parse(username, &Query::new()).is_ok());
    }

    // Seasons / schedules
    assert!(QueryCurrentAnimeSeasonCommand::parse(&Query::new()).is_ok());
    assert!(QueryUpcomingAnimeSeasonCommand::parse(&Query::new()).is_ok());
    assert!(QuerySpecificAnimeSeasonCommand::parse(2024, "winter", &Query::new()).is_ok());
    assert!(QueryAnimeSeasonListCommand::parse(&Query::new()).is_ok());
    assert!(QueryAnimeSchedulesCommand::parse(None, &Query::new()).is_ok());

    // Top
    assert!(QueryTopAnimeItemsCommand::parse(&Query::new()).is_ok());
    assert!(QueryTopMangaItemsCommand::parse(&Query::new()).is_ok());
    assert!(QueryTopCharactersCommand::parse(&Query::new()).is_ok());
    assert!(QueryTopPeopleCommand::parse(&Query::new()).is_ok());
    assert!(QueryTopReviewsCommand::parse(&Query::new()).is_ok());

    // Reviews / recommendations
    assert!(QueryAnimeReviewsCommand::parse(&Query::new()).is_ok());
    assert!(QueryMangaReviewsCommand::parse(&Query::new()).is_ok());
    assert!(QueryAnimeRecommendationsCommand::parse(&Query::new()).is_ok());
    assert!(QueryMangaRecommendationsCommand::parse(&Query::new()).is_ok());

    // Watch / random
    assert!(QueryRecentlyAddedEpisodesCommand::parse(&Query::new()).is_ok());
    assert!(QueryPopularEpisodesCommand::parse(&Query::new()).is_ok());
    assert!(QueryRecentlyAddedPromoVideosCommand::parse(&Query::new()).is_ok());
    assert!(QueryPopularPromoVideosCommand::parse(&Query::new()).is_ok());
    assert!(QueryRecentlyOnlineUsersCommand::parse(&Query::new()).is_ok());
    assert!(QueryRandomAnimeCommand::parse(&Query::new()).is_ok());
    assert!(QueryRandomMangaCommand::parse(&Query::new()).is_ok());
    assert!(QueryRandomCharacterCommand::parse(&Query::new()).is_ok());
    assert!(QueryRandomPersonCommand::parse(&Query::new()).is_ok());
    assert!(QueryRandomUserCommand::parse(&Query::new()).is_ok());
}

#[test]
fn fingerprint_trait_is_implemented_for_all_fingerprint_commands() {
    fn assert_fingerprint<T: HasRequestFingerprint>(command: &T) {
        assert!(command.request_fingerprint("/v4/anime").starts_with("request:"));
    }
    assert_fingerprint(&QueryCurrentAnimeSeasonCommand::parse(&Query::new()).unwrap());
    assert_fingerprint(&QueryAnimeSchedulesCommand::parse(None, &Query::new()).unwrap());
    assert_fingerprint(&QueryAnimeSeasonListCommand::parse(&Query::new()).unwrap());
    assert_fingerprint(&UsersSearchCommand::parse(&Query::new()).unwrap());
    assert_fingerprint(&QueryAnimeRecommendationsCommand::parse(&Query::new()).unwrap());
    assert_fingerprint(&QueryPopularEpisodesCommand::parse(&Query::new()).unwrap());
    assert_fingerprint(
        &QueryAnimeListOfUserCommand::parse("nekomata", &Query::new()).unwrap(),
    );
}

#[test]
fn review_request_params_apply_handler_defaults() {
    let command = QueryAnimeReviewsCommand::parse(&Query::new()).unwrap();
    let params = command.review_request_params();
    assert_eq!(params.sort, kuukan_core::enums::MediaReviewsSort::MostVoted);
    assert!(!params.spoilers);
    assert!(!params.preliminary);
    assert_eq!(params.page, 1);

    let query = Query::from_pairs([
        ("sort", "oldest"),
        ("spoilers", "true"),
        ("preliminary", "true"),
        ("page", "3"),
    ]);
    let params = QueryMangaReviewsCommand::parse(&query)
        .unwrap()
        .review_request_params();
    assert_eq!(params.sort, kuukan_core::enums::MediaReviewsSort::Oldest);
    assert!(params.spoilers);
    assert!(params.preliminary);
    assert_eq!(params.page, 3);
}

#[test]
fn probe_parity_messages() {
    // Captured PHP outputs for cases that are not covered in the family tests.

    // `sfw=TRUE` is not converted by PreparesData and fails the boolean rule.
    let messages = bag(AnimeSearchCommand::parse(&Query::from_pairs([("sfw", "TRUE")])).unwrap_err());
    assert_eq!(messages["sfw"], serde_json::json!(["The sfw field must be true or false."]));

    // `limit=1e3` fails `integer`; the custom max rule uses `intval()`.
    let messages = bag(AnimeSearchCommand::parse(&Query::from_pairs([("limit", "1e3")])).unwrap_err());
    assert_eq!(
        messages["limit"],
        serde_json::json!([
            "The limit must be an integer.",
            "Value 1e3 is higher than the configured '25' max value."
        ])
    );

    // `limit=025` is numeric but not a FILTER_VALIDATE_INT value.
    let messages = bag(AnimeSearchCommand::parse(&Query::from_pairs([("limit", "025")])).unwrap_err());
    assert_eq!(messages["limit"], serde_json::json!(["The limit must be an integer."]));

    // `producer=0` fails only Min(1); `abc` fails numeric + integer.
    let messages = bag(AnimeSearchCommand::parse(&Query::from_pairs([("producer", "0")])).unwrap_err());
    assert_eq!(messages["producer"], serde_json::json!(["The producer must be at least 1."]));
    let messages = bag(AnimeSearchCommand::parse(&Query::from_pairs([("producer", "abc")])).unwrap_err());
    assert_eq!(
        messages["producer"],
        serde_json::json!([
            "The producer must be a number.",
            "The producer must be an integer."
        ])
    );

    // Between is checked before numeric, and uses the string length when the
    // value is not numeric.
    let messages = bag(AnimeSearchCommand::parse(&Query::from_pairs([("min_score", "abcdefghijk")])).unwrap_err());
    assert_eq!(
        messages["min_score"],
        serde_json::json!([
            "The min score must be between 0 and 10.",
            "The min score must be a number."
        ])
    );
    let messages = bag(AnimeSearchCommand::parse(&Query::from_pairs([("max_score", "abcdefghijk")])).unwrap_err());
    assert_eq!(
        messages["max_score"],
        serde_json::json!([
            "The max score must be between 1 and 10.",
            "The max score must be a number."
        ])
    );

    // `min_score=abc` passes `between` because 3 is within [0, 10].
    let messages = bag(AnimeSearchCommand::parse(&Query::from_pairs([("min_score", "abc")])).unwrap_err());
    assert_eq!(messages["min_score"], serde_json::json!(["The min score must be a number."]));

    // `min_score=abc` passes `lte` because `getSize('abc') == 3` is smaller
    // than the numeric max.
    let messages = bag(
        AnimeSearchCommand::parse(&Query::from_pairs([("min_score", "abc"), ("max_score", "5")]))
            .unwrap_err(),
    );
    assert_eq!(
        messages["min_score"],
        serde_json::json!(["The min score must be a number."])
    );
    assert!(messages.get("max_score").is_none());

    // Cross `lte`/`gte` use the *other* value for the `:value` placeholder.
    let messages = bag(
        AnimeSearchCommand::parse(&Query::from_pairs([("min_score", "5"), ("max_score", "abc")]))
            .unwrap_err(),
    );
    assert_eq!(
        messages["min_score"],
        serde_json::json!(["The min score must be less than or equal to 3."])
    );
    assert_eq!(
        messages["max_score"],
        serde_json::json!([
            "The max score must be a number.",
            "The max score must be greater than or equal to 5."
        ])
    );

    // `q` is truncated by `Max(255)` before the string rule.
    let messages = bag(AnimeSearchCommand::parse(&Query::from_pairs([("q", "a".repeat(256))])).unwrap_err());
    assert_eq!(
        messages["q"],
        serde_json::json!(["The q must not be greater than 255 characters."])
    );

    // `letter` whitespace-only values skip the rules.
    assert!(AnimeSearchCommand::parse(&Query::from_pairs([("letter", " ")])).is_ok());
}

#[test]
fn date_cross_rule_message_order_matches_php() {
    // `before_or_equal` runs before `date_format`; an unparsable counterpart
    // fails the comparison of the other field as well.
    let messages = bag(
        AnimeSearchCommand::parse(&Query::from_pairs([
            ("start_date", "garbage"),
            ("end_date", "2020-01-01"),
        ]))
        .unwrap_err(),
    );
    assert_eq!(
        messages["start_date"],
        serde_json::json!([
            "The start date must be a date before or equal to end date.",
            "The start date does not match the format Y-m-d."
        ])
    );
    assert_eq!(
        messages["end_date"],
        serde_json::json!([
            "The end date must be a date after or equal to start date."
        ])
    );

    let messages = bag(
        AnimeSearchCommand::parse(&Query::from_pairs([
            ("start_date", "2020-01-01"),
            ("end_date", "garbage"),
        ]))
        .unwrap_err(),
    );
    assert_eq!(
        messages["start_date"],
        serde_json::json!([
            "The start date must be a date before or equal to end date."
        ])
    );
    assert_eq!(
        messages["end_date"],
        serde_json::json!([
            "The end date must be a date after or equal to start date.",
            "The end date does not match the format Y-m-d."
        ])
    );

    // Slash dates fail `date_format` but Carbon still parses them for the
    // cross-field comparison.
    let messages = bag(
        AnimeSearchCommand::parse(&Query::from_pairs([
            ("start_date", "2020/01/01"),
            ("end_date", "2019-01-01"),
        ]))
        .unwrap_err(),
    );
    assert_eq!(
        messages["start_date"],
        serde_json::json!([
            "The start date must be a date before or equal to end date.",
            "The start date does not match the format Y-m-d."
        ])
    );
    assert_eq!(
        messages["end_date"],
        serde_json::json!([
            "The end date must be a date after or equal to start date."
        ])
    );
}

#[test]
fn whitespace_only_values_follow_php_casts() {
    // Blank strings skip the rules and then fail the int/float cast.
    for (field, value) in [("limit", " "), ("page", " "), ("score", " ")] {
        let error = AnimeSearchCommand::parse(&Query::from_pairs([(field, value)]))
            .unwrap_err();
        assert_eq!(error.status(), 500, "{field}");
    }
    let error =
        AnimeSearchCommand::parse(&Query::from_pairs([("min_score", " "), ("max_score", " ") ]))
            .unwrap_err();
    assert_eq!(error.status(), 500);

    // Blank date values hit the explicit `Required` attribute.
    let messages = bag(
        AnimeSearchCommand::parse(&Query::from_pairs([("start_date", " ")]))
            .unwrap_err(),
    );
    assert_eq!(
        messages["start_date"],
        serde_json::json!(["The start date field is required."])
    );

    // An empty string is dropped by `PreparesData` for media searches.
    assert!(AnimeSearchCommand::parse(&Query::from_pairs([
        ("start_date", ""),
        ("end_date", "")
    ]))
    .is_ok());

    // Whitespace parses as *now* for the cross-field comparison.
    let messages = bag(
        AnimeSearchCommand::parse(&Query::from_pairs([
            ("start_date", " "),
            ("end_date", "2020-01-01"),
        ]))
        .unwrap_err(),
    );
    assert_eq!(
        messages["start_date"],
        serde_json::json!(["The start date field is required."])
    );
    assert_eq!(
        messages["end_date"],
        serde_json::json!([
            "The end date must be a date after or equal to start date."
        ])
    );
}

#[test]
fn empty_values_fail_the_cast_in_non_prepares_data_classes() {
    // PHP raises a `TypeError` assigning `""` to an int property.
    let query = Query::from_pairs([("page", "")]);
    assert_eq!(
        AnimeEpisodesLookupCommand::parse(1, &query).unwrap_err().status(),
        500
    );
    assert_eq!(
        QueryRecentlyAddedPromoVideosCommand::parse(&query)
            .unwrap_err()
            .status(),
        500
    );

    // PHP `EnumCast::cast()` throws `CannotCastEnum` for `""`.
    let query = Query::from_pairs([("filter", "")]);
    assert_eq!(
        GenreListCommand::parse(&query).unwrap_err().status(),
        500
    );
    assert_eq!(
        AnimeForumLookupCommand::parse(1, &query).unwrap_err().status(),
        500
    );
    let query = Query::from_pairs([("type", "")]);
    assert_eq!(
        UserHistoryLookupCommand::parse("nekomata", &query)
            .unwrap_err()
            .status(),
        500
    );

    // `PreparesData` classes drop empty `Optional` values instead.
    let query = Query::from_pairs([("minAge", ""), ("gender", ""), ("location", "")]);
    assert!(UsersSearchCommand::parse(&query).is_ok());
}

#[test]
fn probe_parity_user_lists() {
    // `year=abc` runs `numeric` then `min` (string length 3).
    let messages = bag(
        QueryAnimeListOfUserCommand::parse("nekomata", &Query::from_pairs([("year", "abc")]))
            .unwrap_err(),
    );
    assert_eq!(
        messages["year"],
        serde_json::json!([
            "The year must be a number.",
            "The year must be at least 1500."
        ])
    );

    // `year=1000` fails Min(1500).
    let messages = bag(
        QueryAnimeListOfUserCommand::parse("nekomata", &Query::from_pairs([("year", "1000")]))
            .unwrap_err(),
    );
    assert_eq!(messages["year"], serde_json::json!(["The year must be at least 1500."]));

    // `aired_to` is validated even when `aired_from` is empty.
    let messages = bag(
        QueryAnimeListOfUserCommand::parse(
            "nekomata",
            &Query::from_pairs([("aired_from", "2010-01-01"), ("aired_to", "")]),
        )
        .unwrap_err(),
    );
    assert_eq!(
        messages["aired_to"],
        serde_json::json!(["The aired to field is required."])
    );

    // Empty `order_by` on the non-`PreparesData` list command fails the cast.
    let error = QueryAnimeListOfUserCommand::parse(
        "nekomata",
        &Query::from_pairs([("order_by", "")]),
    )
    .unwrap_err();
    assert_eq!(error.status(), 500);

    // `magazine` numeric rule only.
    let messages = bag(
        QueryMangaListOfUserCommand::parse("nekomata", &Query::from_pairs([("magazine", "abc")]))
            .unwrap_err(),
    );
    assert_eq!(messages["magazine"], serde_json::json!(["The magazine must be a number."]));

    // `q` is capped at 255 characters on the list commands as well.
    let messages = bag(
        QueryAnimeListOfUserCommand::parse(
            "nekomata",
            &Query::from_pairs([("q", "a".repeat(256))]),
        )
        .unwrap_err(),
    );
    assert_eq!(
        messages["q"],
        serde_json::json!(["The q must not be greater than 255 characters."])
    );
}
