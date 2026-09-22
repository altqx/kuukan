//! `Jikan\Helper\Constants` frozen as `pub const`s.
//!
//! Verbatim port of `/tmp/opencode/jikan-php/src/Helper/Constants.php`
//! (jikan-php v4.0.12). The values are mal_id ids: anime/manga genres, themes
//! and demographics share the `GENRE_*` ranges (e.g. `SCHOOL`/`MUSIC` are
//! themes, `SEINEN`/`JOSEI`/`KIDS` are demographics), MAL search/order codes,
//! user-list status/order codes and review sort labels.
//!
//! Notes preserved from the PHP source: several constants are MAL renames and
//! kept as aliases (`GENRE_ANIME_CARS` == `RACING`), and some ids were removed
//! or merged by MAL (e.g. `GENRE_ANIME_MAGIC`) but kept for reference.

// ---------------------------------------------------------------------------
// Base URLs
// ---------------------------------------------------------------------------

/// `Constants::BASE_URL`.
pub const BASE_URL: &str = "https://myanimelist.net";
/// `Constants::CDN_URL`.
pub const CDN_URL: &str = "https://cdn.myanimelist.net";

// ---------------------------------------------------------------------------
// Seasons
// ---------------------------------------------------------------------------

/// `Constants::SEASONS`.
pub const SEASONS: [&str; 4] = ["Winter", "Spring", "Summer", "Fall"];

/// `Constants::WINTER`.
pub const WINTER: &str = "winter";
/// `Constants::SPRING`.
pub const SPRING: &str = "spring";
/// `Constants::SUMMER`.
pub const SUMMER: &str = "summer";
/// `Constants::FALL`.
pub const FALL: &str = "fall";

// ---------------------------------------------------------------------------
// Anime genres / themes / demographics
// ---------------------------------------------------------------------------

/// `Constants::GENRE_ANIME_HENTAI`.
pub const GENRE_ANIME_HENTAI: i32 = 12;
/// `Constants::GENRE_ANIME_KIDS`.
pub const GENRE_ANIME_KIDS: i32 = 15;
/// `Constants::GENRE_ANIME_EROTICA`.
pub const GENRE_ANIME_EROTICA: i32 = 49;

// ---------------------------------------------------------------------------
// User manga list order_by codes
// ---------------------------------------------------------------------------

/// `Constants::USER_LIST_SORT_DESCENDING`.
pub const USER_LIST_SORT_DESCENDING: i32 = 1;
/// `Constants::USER_LIST_SORT_ASCENDING`.
pub const USER_LIST_SORT_ASCENDING: i32 = -1;

/// `Constants::REVIEWS_SORT_MOST_VOTED`.
pub const REVIEWS_SORT_MOST_VOTED: &str = "mostvoted";
/// `Constants::REVIEWS_SORT_OLDEST`.
pub const REVIEWS_SORT_OLDEST: &str = "oldest";
/// `Constants::REVIEWS_SORT_NEWEST`.
pub const REVIEWS_SORT_NEWEST: &str = "newest";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_and_seasons() {
        assert_eq!(BASE_URL, "https://myanimelist.net");
        assert_eq!(CDN_URL, "https://cdn.myanimelist.net");
        assert_eq!(SEASONS, ["Winter", "Spring", "Summer", "Fall"]);
        assert_eq!(
            [WINTER, SPRING, SUMMER, FALL],
            ["winter", "spring", "summer", "fall"]
        );
    }

    /// The three genre ids the SFW filter keys on. These are the only genre
    /// ids kuukan hardcodes; every other genre reaches it from MAL at runtime.
    #[test]
    fn sfw_filter_genre_ids() {
        assert_eq!(GENRE_ANIME_HENTAI, 12);
        assert_eq!(GENRE_ANIME_EROTICA, 49);
        assert_eq!(GENRE_ANIME_KIDS, 15);
    }

    #[test]
    fn sort_values_match_the_mal_query_parameters() {
        assert_eq!(USER_LIST_SORT_DESCENDING, 1);
        assert_eq!(USER_LIST_SORT_ASCENDING, -1);
        assert_eq!(REVIEWS_SORT_MOST_VOTED, "mostvoted");
        assert_eq!(REVIEWS_SORT_NEWEST, "newest");
        assert_eq!(REVIEWS_SORT_OLDEST, "oldest");
    }
}
