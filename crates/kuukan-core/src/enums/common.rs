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
            MediaReviewsSort::parse("mostVoted"),
            Some(MediaReviewsSort::MostVoted)
        );
        assert_eq!(
            MediaReviewsSort::parse("mostvoted"),
            Some(MediaReviewsSort::MostVoted)
        );
        assert_eq!(
            MediaReviewsSort::parse("MOSTVOTED"),
            Some(MediaReviewsSort::MostVoted)
        );
        assert_eq!(
            MediaReviewsSort::parse("newest"),
            Some(MediaReviewsSort::Newest)
        );
        // `suggested` only exists in labels(); no docblock method => rejected.
        assert_eq!(MediaReviewsSort::parse("suggested"), None);
    }

    #[test]
    fn gender_labels_are_mal_codes_rejected_as_input() {
        assert_eq!(Gender::Any.as_str(), "-1");
        assert_eq!(Gender::Male.as_str(), "1");
        assert_eq!(Gender::Nonbinary.as_str(), "3");
        assert_eq!(Gender::parse("nonbinary"), Some(Gender::Nonbinary));
        assert_eq!(Gender::parse("-1"), None);
        assert_eq!(Gender::parse("1"), None);
    }

    #[test]
    fn genre_filter_and_forum_filters_match_php() {
        assert_eq!(
            GenreFilter::parse("explicit_genres"),
            Some(GenreFilter::ExplicitGenres)
        );
        assert_eq!(
            GenreFilter::parse("demographics"),
            Some(GenreFilter::Demographics)
        );
        assert_eq!(
            AnimeForumFilter::parse("episode"),
            Some(AnimeForumFilter::Episode)
        );
        assert_eq!(
            MangaForumFilter::parse("chapters"),
            Some(MangaForumFilter::Chapters)
        );
        assert_eq!(AnimeForumFilter::parse("episodes"), None);
    }
}
