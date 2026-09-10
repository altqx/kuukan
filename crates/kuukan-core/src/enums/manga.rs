//! Manga-related enums, ported from `App\Enums\*Enum.php`.

php_enum! {
    /// PHP `App\Enums\MangaTypeEnum` (`type` query parameter).
    pub enum MangaType = "App\\Enums\\MangaTypeEnum" {
        Manga => "manga" => "Manga",
        Novel => "novel" => "Novel",
        LightNovel => "lightnovel" => "Light Novel",
        OneShot => "oneshot" => "One-shot",
        Doujin => "doujin" => "Doujinshi",
        Manhwa => "manhwa" => "Manhwa",
        Manhua => "manhua" => "Manhua",
    }
}

php_enum! {
    /// PHP `App\Enums\MangaStatusEnum` (`status` query parameter).
    pub enum MangaStatus = "App\\Enums\\MangaStatusEnum" {
        Publishing => "publishing" => "Publishing",
        Complete => "complete" => "Finished",
        Hiatus => "hiatus" => "On Hiatus",
        Discontinued => "discontinued" => "Discontinued",
        Upcoming => "upcoming" => "Not yet published",
    }
}

php_enum! {
    /// PHP `App\Enums\MangaOrderByEnum` (`order_by` query parameter).
    ///
    /// `start_date`/`end_date` map to the MAL sort keys `published.from` /
    /// `published.to`; those labels are rejected as input.
    pub enum MangaOrderBy = "App\\Enums\\MangaOrderByEnum" {
        MalId => "mal_id" => "mal_id",
        Title => "title" => "title",
        StartDate => "start_date" => "published.from",
        EndDate => "end_date" => "published.to",
        Chapters => "chapters" => "chapters",
        Volumes => "volumes" => "volumes",
        Score => "score" => "score",
        ScoredBy => "scored_by" => "scored_by",
        Rank => "rank" => "rank",
        Popularity => "popularity" => "popularity",
        Members => "members" => "members",
        Favorites => "favorites" => "favorites",
    }
}

php_enum! {
    /// PHP `App\Enums\TopMangaFilterEnum` (`filter` on `/top/manga`).
    pub enum TopMangaFilter = "App\\Enums\\TopMangaFilterEnum" {
        Publishing => "publishing" => "publishing",
        Upcoming => "upcoming" => "upcoming",
        ByPopularity => "bypopularity" => "bypopularity",
        Favorite => "favorite" => "favorite",
    }
}

php_enum! {
    /// PHP `App\Enums\MangaListStatusEnum` (`status` on a user's manga list).
    ///
    /// Labels are MAL list-status codes rendered as strings.
    pub enum MangaListStatus = "App\\Enums\\MangaListStatusEnum" {
        All => "all" => "7",
        Reading => "reading" => "1",
        Completed => "completed" => "2",
        OnHold => "onhold" => "3",
        Dropped => "dropped" => "4",
        PlanToRead => "plantoread" => "6",
    }
}

php_enum! {
    /// PHP `App\Enums\UserMangaListOrderByEnum` (`order_by`/`order_by2` on a
    /// user's manga list).
    ///
    /// **Unresolvable in PHP.** `labels()` maps both `progress` and
    /// `chapters_read` to `USER_MANGA_LIST_ORDER_BY_CHAPTERS` (9) and both
    /// `finished_date` and `started_date` to `2`, so spatie/enum throws
    /// `DuplicateLabelsException` from `resolveDefinition()`: every input
    /// fails validation in PHP, so `parse()` rejects everything here too.
    /// The labels below are the declared (never reachable) mapping.
    pub enum UserMangaListOrderBy = "App\\Enums\\UserMangaListOrderByEnum", php_unresolvable {
        Title => "title" => "1",
        StartedDate => "started_date" => "3",
        Score => "score" => "4",
        LastUpdated => "last_updated" => "5",
        Priority => "priority" => "8",
        Progress => "progress" => "9",
        ChaptersRead => "chapters_read" => "9",
        VolumesRead => "volumes_read" => "10",
        Type => "type" => "11",
        PublishStart => "publish_start" => "12",
        PublishEnd => "publish_end" => "13",
        Status => "status" => "14",
    }
}

php_enum! {
    /// PHP `App\Enums\UserMangaListStatusFilterEnum` (`publishing_status` on
    /// a user's manga list).
    ///
    /// **Unresolvable in PHP.** The docblock cases are `publishing`,
    /// `finished`, `complete`, `to_be_published`, `not_yet_published`, `tba`
    /// and `nya`, but `labels()` declares `finished`/`complete` => 2,
    /// `to_be_aired`/`not_yet_aired`/`tba`/`nya` => 3 plus dead `airing` and
    /// `to_be_aired` keys; the duplicate labels make spatie/enum throw
    /// `DuplicateLabelsException`, so every input is a validation error in
    /// PHP and `parse()` rejects everything here. The labels below are the
    /// declared (never reachable) mapping; `publishing`,
    /// `to_be_published` and `not_yet_published` have no explicit label and
    /// would fall back to their index.
    pub enum UserMangaListStatusFilter = "App\\Enums\\UserMangaListStatusFilterEnum", php_unresolvable {
        Publishing => "publishing" => "publishing",
        Finished => "finished" => "2",
        Complete => "complete" => "2",
        ToBePublished => "to_be_published" => "to_be_published",
        NotYetPublished => "not_yet_published" => "not_yet_published",
        Tba => "tba" => "3",
        Nya => "nya" => "3",
    }
}

/// `App\Enums\MangaTypeEnum::from()` as an `Option`.
pub fn parse_manga_type(value: &str) -> Option<MangaType> {
    MangaType::parse(value)
}

/// `App\Enums\MangaStatusEnum::from()` as an `Option`.
pub fn parse_manga_status(value: &str) -> Option<MangaStatus> {
    MangaStatus::parse(value)
}

/// `App\Enums\MangaOrderByEnum::from()` as an `Option`.
pub fn parse_manga_order_by(value: &str) -> Option<MangaOrderBy> {
    MangaOrderBy::parse(value)
}

/// `App\Enums\TopMangaFilterEnum::from()` as an `Option`.
pub fn parse_top_manga_filter(value: &str) -> Option<TopMangaFilter> {
    TopMangaFilter::parse(value)
}

/// `App\Enums\MangaListStatusEnum::from()` as an `Option`.
pub fn parse_manga_list_status(value: &str) -> Option<MangaListStatus> {
    MangaListStatus::parse(value)
}

/// `App\Enums\UserMangaListOrderByEnum::from()` as an `Option`.
///
/// Always `None`: the PHP enum cannot be resolved (duplicate labels).
pub fn parse_user_manga_list_order_by(value: &str) -> Option<UserMangaListOrderBy> {
    UserMangaListOrderBy::parse(value)
}

/// `App\Enums\UserMangaListStatusFilterEnum::from()` as an `Option`.
///
/// Always `None`: the PHP enum cannot be resolved (duplicate labels).
pub fn parse_user_manga_list_status_filter(value: &str) -> Option<UserMangaListStatusFilter> {
    UserMangaListStatusFilter::parse(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enums::testutil::assert_enum_roundtrip;

    #[test]
    fn all_resolvable_enums_roundtrip() {
        assert_enum_roundtrip(MangaType::ALL, MangaType::INDEXES, MangaType::LABELS);
        assert_enum_roundtrip(MangaStatus::ALL, MangaStatus::INDEXES, MangaStatus::LABELS);
        assert_enum_roundtrip(
            MangaOrderBy::ALL,
            MangaOrderBy::INDEXES,
            MangaOrderBy::LABELS,
        );
        assert_enum_roundtrip(
            TopMangaFilter::ALL,
            TopMangaFilter::INDEXES,
            TopMangaFilter::LABELS,
        );
        assert_enum_roundtrip(
            MangaListStatus::ALL,
            MangaListStatus::INDEXES,
            MangaListStatus::LABELS,
        );
    }

    #[test]
    fn manga_type_and_status_match_php() {
        assert_eq!(MangaType::LightNovel.as_str(), "Light Novel");
        assert_eq!(parse_manga_type("lightnovel"), Some(MangaType::LightNovel));
        assert_eq!(parse_manga_type("LIGHTNOVEL"), Some(MangaType::LightNovel));
        assert_eq!(parse_manga_type("Light Novel"), None);
        assert_eq!(parse_manga_type("One-shot"), None);

        assert_eq!(MangaStatus::Complete.as_str(), "Finished");
        assert_eq!(parse_manga_status("complete"), Some(MangaStatus::Complete));
        assert_eq!(parse_manga_status("Finished"), None);
        assert_eq!(parse_manga_status("On Hiatus"), None);
    }

    #[test]
    fn manga_order_by_rejects_mal_sort_keys() {
        assert_eq!(
            parse_manga_order_by("start_date"),
            Some(MangaOrderBy::StartDate)
        );
        assert_eq!(MangaOrderBy::StartDate.as_str(), "published.from");
        assert_eq!(MangaOrderBy::EndDate.as_str(), "published.to");
        assert_eq!(parse_manga_order_by("published.from"), None);
        assert_eq!(parse_manga_order_by("published.to"), None);
    }

    #[test]
    fn manga_list_status_uses_mal_codes_as_labels() {
        assert_eq!(MangaListStatus::All.as_str(), "7");
        assert_eq!(MangaListStatus::PlanToRead.as_str(), "6");
        assert_eq!(
            parse_manga_list_status("plantoread"),
            Some(MangaListStatus::PlanToRead)
        );
        assert_eq!(parse_manga_list_status("6"), None);
    }

    #[test]
    fn user_manga_list_order_by_is_unresolvable_in_php() {
        const { assert!(!UserMangaListOrderBy::RESOLVES_IN_PHP) };
        assert_eq!(
            UserMangaListOrderBy::PHP_CLASS,
            "App\\Enums\\UserMangaListOrderByEnum"
        );
        for index in UserMangaListOrderBy::INDEXES {
            assert_eq!(
                parse_user_manga_list_order_by(index),
                None,
                "PHP rejects `{index}`"
            );
        }
        assert!(parse_user_manga_list_order_by("chapters_read").is_none());
    }

    #[test]
    fn user_manga_list_status_filter_is_unresolvable_in_php() {
        const { assert!(!UserMangaListStatusFilter::RESOLVES_IN_PHP) };
        assert_eq!(
            UserMangaListStatusFilter::PHP_CLASS,
            "App\\Enums\\UserMangaListStatusFilterEnum"
        );
        for index in UserMangaListStatusFilter::INDEXES {
            assert_eq!(
                parse_user_manga_list_status_filter(index),
                None,
                "PHP rejects `{index}`"
            );
        }
    }
}
