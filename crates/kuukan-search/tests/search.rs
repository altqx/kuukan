//! End-to-end tests for the Tantivy search port.
//!
//! Synthetic JMS-shaped payloads (10 anime, 5 manga, plus one of each other
//! kind) exercise the PHP filter semantics from `MediaFilters` /
//! `JikanApiSearchableModel` and the response shape of `SearchResult`.

use kuukan_search::schema::EntityKind;
use kuukan_search::{IndexPipeline, SearchIndex, SearchParams, SortDirection};
use serde_json::{json, Value};

fn merge(base: &mut Value, extra: Value) {
    match (base, extra) {
        (Value::Object(base), Value::Object(extra)) => {
            for (key, value) in extra {
                merge(base.entry(key).or_insert(Value::Null), value);
            }
        }
        (base, extra) => *base = extra,
    }
}

/// A synthetic anime payload. `extra` is deep-merged over the defaults.
fn anime(id: u64, title: &str, extra: Value) -> Value {
    let mut payload = json!({
        "mal_id": id,
        "title": title,
        "title_english": title,
        "title_japanese": Value::Null,
        "titles": [{"type": "Default", "title": title}],
        "type": "TV",
        "status": "Finished Airing",
        "rating": "PG-13 - Teens 13 or older",
        "source": "Original",
        "score": Value::Null,
        "scored_by": Value::Null,
        "rank": Value::Null,
        "popularity": Value::Null,
        "members": Value::Null,
        "favorites": Value::Null,
        "episodes": Value::Null,
        "aired": {"from": Value::Null, "to": Value::Null},
        "approved": true,
        "airing": false,
        "season": Value::Null,
        "year": Value::Null,
        "broadcast": {"day": Value::Null},
        "genres": [],
        "themes": [],
        "demographics": [],
        "explicit_genres": [],
        "producers": [],
        "studios": [],
        "licensors": [],
    });
    merge(&mut payload, extra);
    payload
}

/// A synthetic manga payload. `extra` is deep-merged over the defaults.
fn manga(id: u64, title: &str, extra: Value) -> Value {
    let mut payload = json!({
        "mal_id": id,
        "title": title,
        "title_english": title,
        "title_japanese": Value::Null,
        "titles": [{"type": "Default", "title": title}],
        "type": "Manga",
        "status": "Finished",
        "score": Value::Null,
        "scored_by": Value::Null,
        "rank": Value::Null,
        "popularity": Value::Null,
        "members": Value::Null,
        "favorites": Value::Null,
        "chapters": Value::Null,
        "volumes": Value::Null,
        "published": {"from": Value::Null, "to": Value::Null},
        "approved": true,
        "genres": [],
        "themes": [],
        "demographics": [],
        "explicit_genres": [],
        "serializations": [],
    });
    merge(&mut payload, extra);
    payload
}

fn genres(ids: &[u64]) -> Value {
    Value::Array(ids.iter().map(|id| json!({"mal_id": id})).collect())
}

fn anime_fixtures() -> Vec<Value> {
    vec![
        anime(
            1,
            "Cowboy Bebop",
            json!({
                "title_japanese": "カウボーイビバップ",
                "titles": [
                    {"type": "Default", "title": "Cowboy Bebop"},
                    {"type": "Synonym", "title": "Cowboy Bebop: The Movie"}
                ],
                "rating": "R - 17+ (violence & profanity)",
                "score": 8.75,
                "scored_by": 1_000_000,
                "rank": 40,
                "popularity": 45,
                "members": 1_800_000,
                "favorites": 100_000,
                "episodes": 26,
                "aired": {"from": "1998-04-03T00:00:00+00:00", "to": "1999-04-24T00:00:00+00:00"},
                "season": "spring",
                "year": 1998,
                "broadcast": {"day": "Saturdays"},
                "genres": genres(&[1, 24]),
                "themes": genres(&[50]),
                "demographics": genres(&[27]),
                "producers": genres(&[16]),
                "studios": genres(&[14]),
                "licensors": genres(&[119]),
            }),
        ),
        anime(
            2,
            "Boku no Hero Academia",
            json!({
                "status": "Currently Airing",
                "airing": true,
                "score": 7.9,
                "popularity": 10,
                "rank": 100,
                "aired": {"from": "2016-04-03T00:00:00+00:00"},
                "season": "spring",
                "year": 2016,
                "genres": genres(&[1, 27]),
                "themes": genres(&[38]),
                "demographics": genres(&[27]),
                "studios": genres(&[14]),
            }),
        ),
        anime(
            3,
            "Fullmetal Alchemist: Brotherhood",
            json!({
                "rating": "R - 17+ (violence & profanity)",
                "score": 9.1,
                "popularity": 3,
                "rank": 3,
                "aired": {"from": "2009-04-05T00:00:00+00:00", "to": "2010-07-04T00:00:00+00:00"},
                "genres": genres(&[1, 2]),
                "themes": genres(&[27, 38]),
                "demographics": genres(&[27]),
                "studios": genres(&[2]),
            }),
        ),
        anime(
            4,
            "Kimi no Na wa.",
            json!({
                "type": "Movie",
                "score": 8.85,
                "popularity": 2,
                "aired": {"from": "2016-08-26T00:00:00+00:00", "to": "2016-08-26T00:00:00+00:00"},
                "genres": genres(&[1, 2]),
                "themes": genres(&[39]),
                "demographics": genres(&[27]),
                "studios": genres(&[43]),
            }),
        ),
        anime(
            5,
            "Hentai Anime",
            json!({
                "rating": "Rx - Hentai",
                "score": 6.5,
                "genres": genres(&[12]),
                "demographics": genres(&[12]),
            }),
        ),
        anime(
            6,
            "Kodomo no Omocha",
            json!({
                "rating": "PG - Children",
                "score": 7.2,
                "genres": genres(&[1]),
                "demographics": genres(&[15]),
            }),
        ),
        anime(
            7,
            "Upcoming Show",
            json!({
                "status": "Not yet aired",
                "approved": false,
                "popularity": 5000,
            }),
        ),
        anime(8, "Scoreless Movie", json!({"type": "Movie"})),
        anime(
            9,
            "Erotica Show",
            json!({
                "rating": "R+ - Mild Nudity",
                "genres": genres(&[1]),
                "demographics": genres(&[49]),
            }),
        ),
        anime(
            10,
            "Cowboy Bebop: Tengoku no Tobira",
            json!({
                "type": "Movie",
                "score": 8.4,
                "aired": {"from": "2001-09-01T00:00:00+00:00", "to": "2001-09-01T00:00:00+00:00"},
                "genres": genres(&[1, 24]),
                "themes": genres(&[50]),
                "demographics": genres(&[27]),
                "studios": genres(&[14]),
            }),
        ),
    ]
}

fn manga_fixtures() -> Vec<Value> {
    vec![
        manga(
            100,
            "Berserk",
            json!({
                "status": "Publishing",
                "score": 9.4,
                "published": {"from": "1989-08-25T00:00:00+00:00"},
                "genres": genres(&[1, 2]),
                "themes": genres(&[50]),
                "demographics": genres(&[27]),
                "serializations": genres(&[1]),
            }),
        ),
        manga(
            101,
            "One Piece",
            json!({
                "status": "Publishing",
                "score": 9.2,
                "published": {"from": "1997-07-19T00:00:00+00:00"},
                "genres": genres(&[1, 2]),
                "serializations": genres(&[1]),
            }),
        ),
        manga(
            102,
            "Naruto",
            json!({
                "score": 8.7,
                "published": {"from": "1999-09-21T00:00:00+00:00", "to": "2014-11-10T00:00:00+00:00"},
                "genres": genres(&[1, 2]),
                "serializations": genres(&[1]),
            }),
        ),
        manga(
            103,
            "Doujin Manga",
            json!({
                "type": "Doujinshi",
                "score": 6.0,
                "genres": genres(&[12]),
                "demographics": genres(&[12]),
            }),
        ),
        manga(
            104,
            "Kids Manga",
            json!({
                "score": 7.0,
                "genres": genres(&[1]),
                "demographics": genres(&[15]),
            }),
        ),
    ]
}

fn build_index() -> (tempfile::TempDir, IndexPipeline) {
    let dir = tempfile::tempdir().unwrap();
    let index = SearchIndex::open(dir.path()).unwrap();
    let pipeline = IndexPipeline::new(index);
    pipeline
        .rebuild_from_iter(EntityKind::Anime, anime_fixtures().iter())
        .unwrap();
    pipeline
        .rebuild_from_iter(EntityKind::Manga, manga_fixtures().iter())
        .unwrap();
    (dir, pipeline)
}

fn ids(result: &kuukan_search::SearchResult) -> Vec<u64> {
    result
        .items
        .iter()
        .map(|item| item["mal_id"].as_u64().unwrap())
        .collect()
}

fn search(pipeline: &IndexPipeline, params: SearchParams) -> kuukan_search::SearchResult {
    pipeline.index().search(EntityKind::Anime, &params).unwrap()
}

#[test]
fn q_matches_multiple_title_fields() {
    let (_dir, pipeline) = build_index();

    // English/default title.
    let result = search(
        &pipeline,
        SearchParams {
            q: Some("bebop".to_owned()),
            ..SearchParams::default()
        },
    );
    assert_eq!(ids(&result), vec![1, 10]);
    assert_eq!(result.total, 2);

    // Japanese title (`title_japanese`, weight 3).
    let result = search(
        &pipeline,
        SearchParams {
            q: Some("カウボーイビバップ".to_owned()),
            ..SearchParams::default()
        },
    );
    assert_eq!(ids(&result), vec![1]);

    // Synonym from `titles` (type != Default/English/Japanese).
    let result = search(
        &pipeline,
        SearchParams {
            q: Some("the movie".to_owned()),
            ..SearchParams::default()
        },
    );
    assert!(ids(&result).contains(&1));
}

#[test]
fn long_query_uses_fuzzy_distance_one() {
    let (_dir, pipeline) = build_index();
    let result = search(
        &pipeline,
        SearchParams {
            q: Some("bebap".to_owned()),
            ..SearchParams::default()
        },
    );
    assert_eq!(ids(&result), vec![1, 10]);
}

#[test]
fn short_query_disables_fuzzy_and_matches_prefixes() {
    let (_dir, pipeline) = build_index();

    // "cow" would not match token "cowboy" without prefix handling.
    let result = search(
        &pipeline,
        SearchParams {
            q: Some("cow".to_owned()),
            ..SearchParams::default()
        },
    );
    assert_eq!(ids(&result), vec![1, 10]);

    // 2-char query still prefix-matches.
    let result = search(
        &pipeline,
        SearchParams {
            q: Some("co".to_owned()),
            ..SearchParams::default()
        },
    );
    assert_eq!(ids(&result), vec![1, 10]);

    // Short queries do not tolerate typos.
    let result = search(
        &pipeline,
        SearchParams {
            q: Some("caw".to_owned()),
            ..SearchParams::default()
        },
    );
    assert!(result.items.is_empty());
}

#[test]
fn genre_include_is_and_across_ids_or_across_fields() {
    let (_dir, pipeline) = build_index();

    let result = search(
        &pipeline,
        SearchParams {
            genres: Some("1,24".to_owned()),
            ..SearchParams::default()
        },
    );
    assert_eq!(ids(&result), vec![1, 10]);

    // Genre 27 appears as `demographics` (1, 2, 4, 10) and `themes` (3).
    let result = search(
        &pipeline,
        SearchParams {
            genres: Some("27".to_owned()),
            ..SearchParams::default()
        },
    );
    assert_eq!(ids(&result), vec![1, 2, 3, 4, 10]);

    // Theme 50 on 1 and 10.
    let result = search(
        &pipeline,
        SearchParams {
            genres: Some("50".to_owned()),
            ..SearchParams::default()
        },
    );
    assert_eq!(ids(&result), vec![1, 10]);
}

#[test]
fn malformed_genre_ids_match_nothing_like_php_int_cast() {
    let (_dir, pipeline) = build_index();
    // `filterByGenres` casts "abc" to 0 and requires a match for it too.
    let result = search(
        &pipeline,
        SearchParams {
            genres: Some("abc,1".to_owned()),
            ..SearchParams::default()
        },
    );
    assert!(result.items.is_empty());

    // Empty parts never produce a filter.
    let result = search(
        &pipeline,
        SearchParams {
            genres: Some("".to_owned()),
            ..SearchParams::default()
        },
    );
    assert_eq!(result.total, 9);
}

#[test]
fn genre_exclude_removes_matches() {
    let (_dir, pipeline) = build_index();
    let result = search(
        &pipeline,
        SearchParams {
            genres_exclude: Some("24".to_owned()),
            ..SearchParams::default()
        },
    );
    // 10 fixtures - 2 with genre 24 - 1 unapproved default.
    assert_eq!(result.total, 7);
    assert!(!ids(&result).contains(&1));
    assert!(!ids(&result).contains(&10));
}

#[test]
fn score_ranges_and_ignored_bounds() {
    let (_dir, pipeline) = build_index();

    let result = search(
        &pipeline,
        SearchParams {
            min_score: Some(8.5),
            ..SearchParams::default()
        },
    );
    assert_eq!(ids(&result), vec![1, 3, 4]);

    let result = search(
        &pipeline,
        SearchParams {
            max_score: Some(7.0),
            ..SearchParams::default()
        },
    );
    assert_eq!(ids(&result), vec![5]);

    let result = search(
        &pipeline,
        SearchParams {
            min_score: Some(7.0),
            max_score: Some(8.5),
            ..SearchParams::default()
        },
    );
    assert_eq!(ids(&result), vec![2, 6, 10]);

    // min_score=0 / max_score=10 are ignored (jikan-rest#309).
    let baseline = search(&pipeline, SearchParams::default());
    assert_eq!(baseline.total, 9);
    let result = search(
        &pipeline,
        SearchParams {
            min_score: Some(0.0),
            max_score: Some(10.0),
            ..SearchParams::default()
        },
    );
    assert_eq!(result.total, baseline.total);

    // Exact score match.
    let result = search(
        &pipeline,
        SearchParams {
            score: Some(8.75),
            ..SearchParams::default()
        },
    );
    assert_eq!(ids(&result), vec![1]);
}

#[test]
fn type_status_and_rating_filters() {
    let (_dir, pipeline) = build_index();

    let result = search(
        &pipeline,
        SearchParams {
            media_type: Some("Movie".to_owned()),
            ..SearchParams::default()
        },
    );
    assert_eq!(ids(&result), vec![4, 8, 10]);

    let result = search(
        &pipeline,
        SearchParams {
            status: Some("Currently Airing".to_owned()),
            ..SearchParams::default()
        },
    );
    assert_eq!(ids(&result), vec![2]);

    let result = search(
        &pipeline,
        SearchParams {
            media_type: Some("TV".to_owned()),
            status: Some("Finished Airing".to_owned()),
            ..SearchParams::default()
        },
    );
    assert_eq!(ids(&result), vec![1, 3, 5, 6, 9]);

    let result = search(
        &pipeline,
        SearchParams {
            rating: Some("Rx - Hentai".to_owned()),
            ..SearchParams::default()
        },
    );
    assert_eq!(ids(&result), vec![5]);
}

#[test]
fn order_by_score_desc_and_asc_with_null_dates() {
    let (_dir, pipeline) = build_index();

    let result = search(
        &pipeline,
        SearchParams {
            order_by: Some("score".to_owned()),
            sort: Some(SortDirection::Desc),
            ..SearchParams::default()
        },
    );
    let scores: Vec<Option<f64>> = result
        .items
        .iter()
        .map(|item| item["score"].as_f64())
        .collect();
    assert_eq!(
        scores,
        vec![
            Some(9.1),
            Some(8.85),
            Some(8.75),
            Some(8.4),
            Some(7.9),
            Some(7.2),
            Some(6.5),
            None,
            None
        ]
    );

    // Ascending: null/missing scores first (Mongo semantics), mal_id tie-break.
    let result = search(
        &pipeline,
        SearchParams {
            order_by: Some("score".to_owned()),
            sort: Some(SortDirection::Asc),
            ..SearchParams::default()
        },
    );
    assert_eq!(ids(&result)[0], 8);
    assert!(result.items[0]["score"].is_null());
}

#[test]
fn order_by_start_date_maps_from_aired() {
    let (_dir, pipeline) = build_index();
    let result = search(
        &pipeline,
        SearchParams {
            order_by: Some("aired.from".to_owned()),
            ..SearchParams::default()
        },
    );
    let first_with_date = result
        .items
        .iter()
        .find(|item| !item["aired"]["from"].is_null())
        .unwrap();
    assert_eq!(
        first_with_date["mal_id"], 1,
        "1998-04-03 is the earliest start date"
    );
}

#[test]
fn keyword_search_with_order_by_score() {
    let (_dir, pipeline) = build_index();
    let result = search(
        &pipeline,
        SearchParams {
            q: Some("awesome".to_owned()),
            order_by: Some("score".to_owned()),
            sort: Some(SortDirection::Desc),
            ..SearchParams::default()
        },
    );
    assert!(result.items.is_empty());

    let result = search(
        &pipeline,
        SearchParams {
            q: Some("bebop".to_owned()),
            order_by: Some("score".to_owned()),
            sort: Some(SortDirection::Desc),
            ..SearchParams::default()
        },
    );
    assert_eq!(ids(&result), vec![1, 10]);
}

#[test]
fn pagination_shape_matches_laravel() {
    let (_dir, pipeline) = build_index();
    let result = search(
        &pipeline,
        SearchParams {
            limit: Some(3),
            page: Some(2),
            ..SearchParams::default()
        },
    );
    assert_eq!(result.total, 9);
    assert_eq!(result.per_page, 3);
    assert_eq!(result.current_page, 2);
    assert_eq!(result.last_page, 3);
    assert_eq!(ids(&result), vec![4, 5, 6]);

    // Beyond the last page: empty items, same pagination metadata.
    let result = search(
        &pipeline,
        SearchParams {
            limit: Some(3),
            page: Some(99),
            ..SearchParams::default()
        },
    );
    assert!(result.items.is_empty());
    assert_eq!(result.total, 9);
    assert_eq!(result.last_page, 3);

    // Out-of-range limits are clamped to MAX_RESULTS_PER_PAGE (25 default).
    let result = search(
        &pipeline,
        SearchParams {
            limit: Some(1000),
            ..SearchParams::default()
        },
    );
    assert_eq!(result.per_page, 25);
}

#[test]
fn sfw_excludes_adult_ratings_and_genres() {
    let (_dir, pipeline) = build_index();

    let result = search(
        &pipeline,
        SearchParams {
            sfw: Some(true),
            ..SearchParams::default()
        },
    );
    let found = ids(&result);
    assert!(
        !found.contains(&5),
        "hentai (rating/genre/demographic) is excluded"
    );
    assert!(!found.contains(&9), "erotica demographic is excluded");
    assert!(found.contains(&1), "R-rated non-adult entries stay");
    assert_eq!(result.total, 7);
}

#[test]
fn unapproved_flag_includes_pending_entries() {
    let (_dir, pipeline) = build_index();

    let result = search(&pipeline, SearchParams::default());
    assert_eq!(result.total, 9);

    let result = search(
        &pipeline,
        SearchParams {
            unapproved: Some(true),
            ..SearchParams::default()
        },
    );
    assert_eq!(result.total, 10);
    assert!(ids(&result).contains(&7));
}

#[test]
fn kids_scope_includes_and_excludes() {
    let (_dir, pipeline) = build_index();

    let result = search(
        &pipeline,
        SearchParams {
            kids: Some(true),
            ..SearchParams::default()
        },
    );
    assert_eq!(ids(&result), vec![6]);

    let result = search(
        &pipeline,
        SearchParams {
            kids: Some(false),
            ..SearchParams::default()
        },
    );
    assert!(!ids(&result).contains(&6));
    assert_eq!(result.total, 8);
}

#[test]
fn letter_filter_uses_display_name_prefix() {
    let (_dir, pipeline) = build_index();

    let result = search(
        &pipeline,
        SearchParams {
            letter: Some("c".to_owned()),
            ..SearchParams::default()
        },
    );
    assert_eq!(ids(&result), vec![1, 10]);

    let result = search(
        &pipeline,
        SearchParams {
            letter: Some("b".to_owned()),
            ..SearchParams::default()
        },
    );
    assert_eq!(ids(&result), vec![2]);

    // `letter` wins over `q` (the DTO prohibits both).
    let result = search(
        &pipeline,
        SearchParams {
            q: Some("kimi".to_owned()),
            letter: Some("c".to_owned()),
            ..SearchParams::default()
        },
    );
    assert_eq!(ids(&result), vec![1, 10]);
}

#[test]
fn producers_or_across_studios_licensors() {
    let (_dir, pipeline) = build_index();

    let result = search(
        &pipeline,
        SearchParams {
            producer: Some(14),
            ..SearchParams::default()
        },
    );
    assert_eq!(ids(&result), vec![1, 2, 10]);

    let result = search(
        &pipeline,
        SearchParams {
            producers: Some("16,2".to_owned()),
            ..SearchParams::default()
        },
    );
    assert_eq!(ids(&result), vec![1, 3]);
}

#[test]
fn date_ranges_are_inclusive() {
    let (_dir, pipeline) = build_index();

    let result = search(
        &pipeline,
        SearchParams {
            start_date: Some("2016-04-03".to_owned()),
            ..SearchParams::default()
        },
    );
    assert!(ids(&result).contains(&2));
    assert!(!ids(&result).contains(&1));

    // `aired.to` missing (null) never matches an end_date bound.
    let result = search(
        &pipeline,
        SearchParams {
            end_date: Some("2000-01-01".to_owned()),
            ..SearchParams::default()
        },
    );
    assert_eq!(ids(&result), vec![1]);

    // Inclusive: exactly id 4's end date.
    let result = search(
        &pipeline,
        SearchParams {
            end_date: Some("2016-08-26".to_owned()),
            ..SearchParams::default()
        },
    );
    assert!(ids(&result).contains(&4));
}

#[test]
fn empty_q_behaves_like_a_list_query() {
    let (_dir, pipeline) = build_index();
    let result = search(
        &pipeline,
        SearchParams {
            q: Some(String::new()),
            ..SearchParams::default()
        },
    );
    assert_eq!(ids(&result), vec![1, 2, 3, 4, 5, 6, 8, 9, 10]);
}

#[test]
fn payload_roundtrips_verbatim() {
    let (_dir, pipeline) = build_index();
    let expected = anime_fixtures()
        .into_iter()
        .find(|payload| payload["mal_id"] == 1)
        .unwrap();
    let result = search(
        &pipeline,
        SearchParams {
            q: Some("bebop".to_owned()),
            ..SearchParams::default()
        },
    );
    let stored = result
        .items
        .iter()
        .find(|item| item["mal_id"] == 1)
        .unwrap();
    assert_eq!(*stored, expected);
}

#[test]
fn manga_genres_magazines_and_sfw() {
    let (_dir, pipeline) = build_index();

    let search_manga =
        |params: SearchParams| pipeline.index().search(EntityKind::Manga, &params).unwrap();

    let result = search_manga(SearchParams::default());
    assert_eq!(result.total, 5);

    // `serializations` feeds the `magazines` filter/order field.
    let result = search_manga(SearchParams {
        magazines: Some("1".to_owned()),
        ..SearchParams::default()
    });
    assert_eq!(ids(&result), vec![100, 101, 102]);

    let result = search_manga(SearchParams {
        magazine: Some(999),
        ..SearchParams::default()
    });
    assert!(result.items.is_empty());

    // sfw excludes Doujinshi + hentai demographics/genres.
    let result = search_manga(SearchParams {
        sfw: Some(true),
        ..SearchParams::default()
    });
    let found = ids(&result);
    assert!(!found.contains(&103));
    assert_eq!(result.total, 4);

    let result = search_manga(SearchParams {
        q: Some("piece".to_owned()),
        ..SearchParams::default()
    });
    assert_eq!(ids(&result), vec![101]);

    let result = search_manga(SearchParams {
        genres: Some("50".to_owned()),
        ..SearchParams::default()
    });
    assert_eq!(ids(&result), vec![100]);

    let result = search_manga(SearchParams {
        start_date: Some("1990-01-01".to_owned()),
        ..SearchParams::default()
    });
    assert_eq!(ids(&result), vec![101, 102]);
}

#[test]
fn character_person_club_producer_magazine_indexes() {
    let dir = tempfile::tempdir().unwrap();
    let index = SearchIndex::open(dir.path()).unwrap();
    let pipeline = IndexPipeline::new(index);

    let character = json!({"mal_id": 1, "name": "Spike Spiegel", "name_kanji": "スパイク・スピーゲル", "favorites": 42});
    let person = json!({"mal_id": 2, "name": "Shinichiro Watanabe", "given_name": "Shinichiro", "family_name": "Watanabe", "alternate_names": ["Watanabe Shinichiro"], "favorites": 7, "birthday": "1965-10-27T00:00:00+00:00"});
    let club = json!({"mal_id": 3, "name": "Anime Club", "category": "Anime", "access": "public", "members": 12, "created": "2007-01-01T00:00:00+00:00"});
    let private_club = json!({"mal_id": 4, "name": "Secret Society", "category": "Other", "access": "private", "members": 3, "created": "2008-01-01T00:00:00+00:00"});
    let producer = json!({"mal_id": 5, "titles": [{"type": "Default", "title": "Sunrise"}], "url": "https://myanimelist.net/anime/producer/14", "favorites": 5, "count": 100, "established": "1972-09-01T00:00:00+00:00"});
    let magazine = json!({"mal_id": 6, "name": "Young Animal", "count": 200});

    pipeline
        .rebuild_from_iter(EntityKind::Character, [&character])
        .unwrap();
    pipeline
        .rebuild_from_iter(EntityKind::Person, [&person])
        .unwrap();
    pipeline
        .rebuild_from_iter(EntityKind::Club, [&club, &private_club])
        .unwrap();
    pipeline
        .rebuild_from_iter(EntityKind::Producer, [&producer])
        .unwrap();
    pipeline
        .rebuild_from_iter(EntityKind::Magazine, [&magazine])
        .unwrap();

    let character_result = pipeline
        .index()
        .search(
            EntityKind::Character,
            &SearchParams {
                q: Some("spike".to_owned()),
                ..SearchParams::default()
            },
        )
        .unwrap();
    assert_eq!(ids(&character_result), vec![1]);

    let character_result = pipeline
        .index()
        .search(
            EntityKind::Character,
            &SearchParams {
                q: Some("スパイク".to_owned()),
                ..SearchParams::default()
            },
        )
        .unwrap();
    assert_eq!(ids(&character_result), vec![1]);

    let person_result = pipeline
        .index()
        .search(
            EntityKind::Person,
            &SearchParams {
                q: Some("watanabe".to_owned()),
                ..SearchParams::default()
            },
        )
        .unwrap();
    assert_eq!(ids(&person_result), vec![2]);

    let person_result = pipeline
        .index()
        .search(
            EntityKind::Person,
            &SearchParams {
                order_by: Some("birthday".to_owned()),
                ..SearchParams::default()
            },
        )
        .unwrap();
    assert_eq!(ids(&person_result), vec![2]);

    let club_result = pipeline
        .index()
        .search(
            EntityKind::Club,
            &SearchParams {
                letter: Some("a".to_owned()),
                category: Some("Anime".to_owned()),
                ..SearchParams::default()
            },
        )
        .unwrap();
    assert_eq!(ids(&club_result), vec![3]);

    // `type` maps to the `access` field for clubs.
    let club_result = pipeline
        .index()
        .search(
            EntityKind::Club,
            &SearchParams {
                media_type: Some("public".to_owned()),
                ..SearchParams::default()
            },
        )
        .unwrap();
    assert_eq!(ids(&club_result), vec![3]);

    let club_result = pipeline
        .index()
        .search(
            EntityKind::Club,
            &SearchParams {
                media_type: Some("private".to_owned()),
                ..SearchParams::default()
            },
        )
        .unwrap();
    assert_eq!(ids(&club_result), vec![4]);

    let producer_result = pipeline
        .index()
        .search(
            EntityKind::Producer,
            &SearchParams {
                q: Some("sunrise".to_owned()),
                ..SearchParams::default()
            },
        )
        .unwrap();
    assert_eq!(ids(&producer_result), vec![5]);

    let magazine_result = pipeline
        .index()
        .search(
            EntityKind::Magazine,
            &SearchParams {
                letter: Some("y".to_owned()),
                order_by: Some("count".to_owned()),
                sort: Some(SortDirection::Desc),
                ..SearchParams::default()
            },
        )
        .unwrap();
    assert_eq!(ids(&magazine_result), vec![6]);
}
