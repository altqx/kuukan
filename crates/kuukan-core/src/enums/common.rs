//! Enums shared across media types: sort directions, reviews, genres,
//! genders and forum filters.

php_enum! {
    /// PHP `App\Enums\SortDirection` (`sort` query parameter).
    pub enum SortDirection = "App\\Enums\\SortDirection" {
        Asc => "asc" => "asc",
        Desc => "desc" => "desc",
    }
}

php_enum! {
    /// PHP `App\Enums\ReviewTypeEnum`.
    pub enum ReviewType = "App\\Enums\\ReviewTypeEnum" {
        Anime => "anime" => "anime",
        Manga => "manga" => "manga",
    }
}

php_enum! {
    /// PHP `App\Enums\TopReviewsTypeEnum` (`type` on `/top/reviews`).
    pub enum TopReviewsType = "App\\Enums\\TopReviewsTypeEnum" {
        Anime => "anime" => "anime",
        Manga => "manga" => "manga",
    }
}

php_enum! {
    /// PHP `App\Enums\MediaReviewsSortEnum` (`sort` on reviews endpoints).
    ///
    /// `labels()` also declares `suggested`, but there is no
    /// `@method static self suggested()` docblock method, so PHP never
    /// defines that case and rejects the input (verified). `mostVoted`'s
    /// label is lowercased by `Jikan\Helper\Constants::REVIEWS_SORT_MOST_VOTED`
    /// but its index keeps the camelCase, so both spellings are accepted.
    pub enum MediaReviewsSort = "App\\Enums\\MediaReviewsSortEnum" {
        MostVoted => "mostVoted" => "mostvoted",
        Newest => "newest" => "newest",
        Oldest => "oldest" => "oldest",
    }
}

php_enum! {
    /// PHP `App\Enums\GenreFilterEnum` (`filter` on genre endpoints).
    pub enum GenreFilter = "App\\Enums\\GenreFilterEnum" {
        Genres => "genres" => "genres",
        ExplicitGenres => "explicit_genres" => "explicit_genres",
        Themes => "themes" => "themes",
        Demographics => "demographics" => "demographics",
    }
}

php_enum! {
    /// PHP `App\Enums\GenderEnum` (`gender` on `/users` search).
    ///
    /// Labels are the `Jikan\Helper\Constants::SEARCH_USER_GENDER_*` codes
    /// rendered as strings; input only accepts the indexes.
    pub enum Gender = "App\\Enums\\GenderEnum" {
        Any => "any" => "-1",
        Male => "male" => "1",
        Female => "female" => "2",
        Nonbinary => "nonbinary" => "3",
    }
}

php_enum! {
    /// PHP `App\Enums\AnimeForumFilterEnum` (`filter` on anime forum).
    pub enum AnimeForumFilter = "App\\Enums\\AnimeForumFilterEnum" {
        All => "all" => "all",
        Episode => "episode" => "episode",
        Other => "other" => "other",
    }
}

php_enum! {
    /// PHP `App\Enums\MangaForumFilterEnum` (`filter` on manga forum).
    pub enum MangaForumFilter = "App\\Enums\\MangaForumFilterEnum" {
        All => "all" => "all",
        Chapters => "chapters" => "chapters",
        Other => "other" => "other",
    }
}

/// `App\Enums\SortDirection::from()` as an `Option`.
pub fn parse_sort_direction(value: &str) -> Option<SortDirection> {
    SortDirection::parse(value)
}

/// `App\Enums\ReviewTypeEnum::from()` as an `Option`.
pub fn parse_review_type(value: &str) -> Option<ReviewType> {
    ReviewType::parse(value)
}

/// `App\Enums\TopReviewsTypeEnum::from()` as an `Option`.
pub fn parse_top_reviews_type(value: &str) -> Option<TopReviewsType> {
    TopReviewsType::parse(value)
}

/// `App\Enums\MediaReviewsSortEnum::from()` as an `Option`.
pub fn parse_media_reviews_sort(value: &str) -> Option<MediaReviewsSort> {
    MediaReviewsSort::parse(value)
}

/// `App\Enums\GenreFilterEnum::from()` as an `Option`.
pub fn parse_genre_filter(value: &str) -> Option<GenreFilter> {
    GenreFilter::parse(value)
}

/// `App\Enums\GenderEnum::from()` as an `Option`.
pub fn parse_gender(value: &str) -> Option<Gender> {
    Gender::parse(value)
}

/// `App\Enums\AnimeForumFilterEnum::from()` as an `Option`.
pub fn parse_anime_forum_filter(value: &str) -> Option<AnimeForumFilter> {
    AnimeForumFilter::parse(value)
}

/// `App\Enums\MangaForumFilterEnum::from()` as an `Option`.
pub fn parse_manga_forum_filter(value: &str) -> Option<MangaForumFilter> {
    MangaForumFilter::parse(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enums::testutil::assert_enum_roundtrip;

    #[test]
    fn all_enums_roundtrip() {
        assert_enum_roundtrip(
            SortDirection::ALL,
            SortDirection::INDEXES,
            SortDirection::LABELS,
        );
        assert_enum_roundtrip(ReviewType::ALL, ReviewType::INDEXES, ReviewType::LABELS);
        assert_enum_roundtrip(
            TopReviewsType::ALL,
            TopReviewsType::INDEXES,
            TopReviewsType::LABELS,
        );
        assert_enum_roundtrip(
            MediaReviewsSort::ALL,
            MediaReviewsSort::INDEXES,
            MediaReviewsSort::LABELS,
        );
        assert_enum_roundtrip(GenreFilter::ALL, GenreFilter::INDEXES, GenreFilter::LABELS);
        assert_enum_roundtrip(Gender::ALL, Gender::INDEXES, Gender::LABELS);
        assert_enum_roundtrip(
            AnimeForumFilter::ALL,
            AnimeForumFilter::INDEXES,
            AnimeForumFilter::LABELS,
        );
        assert_enum_roundtrip(
            MangaForumFilter::ALL,
            MangaForumFilter::INDEXES,
            MangaForumFilter::LABELS,
        );
    }

    #[test]
    fn media_reviews_sort_matches_php() {
        assert_eq!(MediaReviewsSort::MostVoted.as_str(), "mostvoted");
        assert_eq!(
            parse_media_reviews_sort("mostVoted"),
            Some(MediaReviewsSort::MostVoted)
        );
        assert_eq!(
            parse_media_reviews_sort("mostvoted"),
            Some(MediaReviewsSort::MostVoted)
        );
        assert_eq!(
            parse_media_reviews_sort("MOSTVOTED"),
            Some(MediaReviewsSort::MostVoted)
        );
        assert_eq!(
            parse_media_reviews_sort("newest"),
            Some(MediaReviewsSort::Newest)
        );
        // `suggested` only exists in labels(); no docblock method => rejected.
        assert_eq!(parse_media_reviews_sort("suggested"), None);
    }

    #[test]
    fn gender_labels_are_mal_codes_rejected_as_input() {
        assert_eq!(Gender::Any.as_str(), "-1");
        assert_eq!(Gender::Male.as_str(), "1");
        assert_eq!(Gender::Nonbinary.as_str(), "3");
        assert_eq!(parse_gender("nonbinary"), Some(Gender::Nonbinary));
        assert_eq!(parse_gender("-1"), None);
        assert_eq!(parse_gender("1"), None);
    }

    #[test]
    fn genre_filter_and_forum_filters_match_php() {
        assert_eq!(
            parse_genre_filter("explicit_genres"),
            Some(GenreFilter::ExplicitGenres)
        );
        assert_eq!(
            parse_genre_filter("demographics"),
            Some(GenreFilter::Demographics)
        );
        assert_eq!(
            parse_anime_forum_filter("episode"),
            Some(AnimeForumFilter::Episode)
        );
        assert_eq!(
            parse_manga_forum_filter("chapters"),
            Some(MangaForumFilter::Chapters)
        );
        assert_eq!(parse_anime_forum_filter("episodes"), None);
    }
}
