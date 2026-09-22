//! Anime-related enums, ported from `App\Enums\*Enum.php`.
//!
//! Each type documents the PHP class it replaces and the exact query-string
//! acceptance of `Spatie\Enum\Enum::from()`.

php_enum! {
    /// PHP `App\Enums\AnimeTypeEnum` (`type` query parameter).
    pub enum AnimeType = "App\\Enums\\AnimeTypeEnum" {
        Tv => "tv" => "TV",
        Movie => "movie" => "Movie",
        Ova => "ova" => "OVA",
        Special => "special" => "Special",
        Ona => "ona" => "ONA",
        Music => "music" => "Music",
        Cm => "cm" => "CM",
        Pv => "pv" => "PV",
        TvSpecial => "tv_special" => "TV Special",
    }
}

php_enum! {
    /// PHP `App\Enums\AnimeStatusEnum` (`status` query parameter).
    pub enum AnimeStatus = "App\\Enums\\AnimeStatusEnum" {
        Airing => "airing" => "Currently Airing",
        Complete => "complete" => "Finished Airing",
        Upcoming => "upcoming" => "Not yet aired",
    }
}

php_enum! {
    /// PHP `App\Enums\AnimeRatingEnum` (`rating` query parameter).
    pub enum AnimeRating = "App\\Enums\\AnimeRatingEnum" {
        G => "g" => "G - All Ages",
        Pg => "pg" => "PG - Children",
        Pg13 => "pg13" => "PG-13 - Teens 13 or older",
        R17 => "r17" => "R - 17+ (violence & profanity)",
        R => "r" => "R+ - Mild Nudity",
        Rx => "rx" => "Rx - Hentai",
    }
}

php_enum! {
    /// PHP `App\Enums\AnimeSeasonEnum` (season route/query parameter).
    ///
    /// PHP `labels()` builds `ucfirst(self::values())`, but `values()` is the
    /// default empty implementation and is never resolved, so the labels stay
    /// the lowercase indexes (`summer`, `spring`, `winter`, `fall`). Verified
    /// against spatie/enum 3.13.0.
    pub enum AnimeSeason = "App\\Enums\\AnimeSeasonEnum" {
        Summer => "summer" => "summer",
        Spring => "spring" => "spring",
        Winter => "winter" => "winter",
        Fall => "fall" => "fall",
    }
}

php_enum! {
    /// PHP `App\Enums\AnimeOrderByEnum` (`order_by` query parameter).
    ///
    /// `start_date`/`end_date` map to the MAL sort keys `aired.from` /
    /// `aired.to`; those are labels only and are rejected as input, exactly
    /// like in PHP.
    pub enum AnimeOrderBy = "App\\Enums\\AnimeOrderByEnum" {
        MalId => "mal_id" => "mal_id",
        Title => "title" => "title",
        StartDate => "start_date" => "aired.from",
        EndDate => "end_date" => "aired.to",
        Episodes => "episodes" => "episodes",
        Score => "score" => "score",
        ScoredBy => "scored_by" => "scored_by",
        Rank => "rank" => "rank",
        Popularity => "popularity" => "popularity",
        Members => "members" => "members",
        Favorites => "favorites" => "favorites",
    }
}

php_enum! {
    /// PHP `App\Enums\TopAnimeFilterEnum` (`filter` on `/top/anime`).
    pub enum TopAnimeFilter = "App\\Enums\\TopAnimeFilterEnum" {
        Airing => "airing" => "airing",
        Upcoming => "upcoming" => "upcoming",
        ByPopularity => "bypopularity" => "bypopularity",
        Favorite => "favorite" => "favorite",
    }
}

php_enum! {
    /// PHP `App\Enums\AnimeScheduleFilterEnum` (`filter` on `/schedules`).
    ///
    /// PHP `labels()` spreads `ucfirst(self::values())` — again with the
    /// default empty `values()` — so weekday labels stay lowercase (a PHP
    /// bug; `DefaultAnimeRepository` queries `broadcast LIKE "monday%"`),
    /// while `other`/`unknown` keep their explicit labels.
    pub enum AnimeScheduleFilter = "App\\Enums\\AnimeScheduleFilterEnum" {
        Monday => "monday" => "monday",
        Tuesday => "tuesday" => "tuesday",
        Wednesday => "wednesday" => "wednesday",
        Thursday => "thursday" => "thursday",
        Friday => "friday" => "friday",
        Saturday => "saturday" => "saturday",
        Sunday => "sunday" => "sunday",
        Other => "other" => "Not scheduled once per week",
        Unknown => "unknown" => "Unknown",
    }
}

impl AnimeScheduleFilter {
    /// PHP `AnimeScheduleFilterEnum::isWeekDay()`.
    pub fn is_week_day(&self) -> bool {
        !matches!(self, Self::Other | Self::Unknown)
    }
}

php_enum! {
    /// PHP `App\Enums\AnimeListStatusEnum` (`status` on a user's anime list).
    ///
    /// Labels are MAL list-status codes rendered as strings.
    pub enum AnimeListStatus = "App\\Enums\\AnimeListStatusEnum" {
        All => "all" => "7",
        Watching => "watching" => "1",
        Completed => "completed" => "2",
        OnHold => "onhold" => "3",
        Dropped => "dropped" => "4",
        PlanToWatch => "plantowatch" => "6",
    }
}

php_enum! {
    /// PHP `App\Enums\AnimeListAiringStatusFilterEnum` (`airing_status` on a
    /// user's anime list).
    ///
    /// **Unresolvable in PHP.** `labels()` maps `finished`/`complete` to `2`
    /// and `to_be_aired`/`not_yet_aired`/`tba`/`nya` to `3`; spatie/enum
    /// throws `DuplicateLabelsException` from `resolveDefinition()`, so
    /// `Enum::from()` fails for every input and `EnumRule::passes()` reports a
    /// validation error. `parse()` therefore rejects everything, matching the
    /// observable PHP behavior. The labels below are the declared (never
    /// reachable) mapping.
    pub enum AnimeListAiringStatusFilter = "App\\Enums\\AnimeListAiringStatusFilterEnum", php_unresolvable {
        Airing => "airing" => "1",
        Finished => "finished" => "2",
        Complete => "complete" => "2",
        ToBeAired => "to_be_aired" => "3",
        NotYetAired => "not_yet_aired" => "3",
        Tba => "tba" => "3",
        Nya => "nya" => "3",
    }
}

php_enum! {
    /// PHP `App\Enums\UserAnimeListOrderByEnum` (`order_by`/`order_by2` on a
    /// user's anime list).
    ///
    /// `labels()` also declares `finished_date => 2`, but there is no
    /// `@method static self finished_date()` in the docblock, so PHP never
    /// defines that case and rejects the input (verified). Labels are MAL
    /// order-by codes.
    pub enum UserAnimeListOrderBy = "App\\Enums\\UserAnimeListOrderByEnum" {
        Title => "title" => "1",
        StartedDate => "started_date" => "3",
        Score => "score" => "4",
        LastUpdated => "last_updated" => "5",
        Type => "type" => "6",
        Rated => "rated" => "8",
        RewatchValue => "rewatch_value" => "9",
        Priority => "priority" => "11",
        EpisodesWatched => "episodes_watched" => "12",
        Storage => "storage" => "13",
        AirStart => "air_start" => "14",
        AirEnd => "air_end" => "15",
        Status => "status" => "16",
    }
}

/// `App\Enums\AnimeListAiringStatusFilterEnum::from()` as an `Option`.
///
#[cfg(test)]
mod tests {
    use super::*;
    use crate::enums::testutil::assert_enum_roundtrip;

    #[test]
    fn all_resolvable_enums_roundtrip() {
        assert_enum_roundtrip(AnimeType::ALL, AnimeType::INDEXES, AnimeType::LABELS);
        assert_enum_roundtrip(AnimeStatus::ALL, AnimeStatus::INDEXES, AnimeStatus::LABELS);
        assert_enum_roundtrip(AnimeRating::ALL, AnimeRating::INDEXES, AnimeRating::LABELS);
        assert_enum_roundtrip(AnimeSeason::ALL, AnimeSeason::INDEXES, AnimeSeason::LABELS);
        assert_enum_roundtrip(
            AnimeOrderBy::ALL,
            AnimeOrderBy::INDEXES,
            AnimeOrderBy::LABELS,
        );
        assert_enum_roundtrip(
            TopAnimeFilter::ALL,
            TopAnimeFilter::INDEXES,
            TopAnimeFilter::LABELS,
        );
        assert_enum_roundtrip(
            AnimeScheduleFilter::ALL,
            AnimeScheduleFilter::INDEXES,
            AnimeScheduleFilter::LABELS,
        );
        assert_enum_roundtrip(
            AnimeListStatus::ALL,
            AnimeListStatus::INDEXES,
            AnimeListStatus::LABELS,
        );
        assert_enum_roundtrip(
            UserAnimeListOrderBy::ALL,
            UserAnimeListOrderBy::INDEXES,
            UserAnimeListOrderBy::LABELS,
        );
    }

    #[test]
    fn anime_type_matches_php_labels_and_casing() {
        assert_eq!(AnimeType::Tv.as_str(), "TV");
        assert_eq!(AnimeType::Tv.index(), "tv");
        assert_eq!(AnimeType::parse("tv"), Some(AnimeType::Tv));
        assert_eq!(AnimeType::parse("TV"), Some(AnimeType::Tv));
        assert_eq!(AnimeType::parse("Tv"), Some(AnimeType::Tv));
        assert_eq!(AnimeType::parse("tV"), Some(AnimeType::Tv));
        assert_eq!(AnimeType::parse("tv_special"), Some(AnimeType::TvSpecial));
        assert_eq!(AnimeType::parse("TV_SPECIAL"), Some(AnimeType::TvSpecial));
        // Labels are not indexes.
        assert_eq!(AnimeType::parse("TV Special"), None);
        assert_eq!(AnimeType::parse("light novel"), None);
    }

    #[test]
    fn anime_status_labels_are_not_accepted_as_input() {
        assert_eq!(AnimeStatus::parse("airing"), Some(AnimeStatus::Airing));
        assert_eq!(AnimeStatus::parse("COMPLETE"), Some(AnimeStatus::Complete));
        assert_eq!(AnimeStatus::parse("Currently Airing"), None);
        assert_eq!(AnimeStatus::parse("Finished Airing"), None);
        assert_eq!(AnimeStatus::parse("Not yet aired"), None);
    }

    #[test]
    fn anime_rating_labels_are_not_accepted_as_input() {
        assert_eq!(AnimeRating::parse("pg13"), Some(AnimeRating::Pg13));
        assert_eq!(AnimeRating::parse("R17"), Some(AnimeRating::R17));
        assert_eq!(AnimeRating::parse("rx"), Some(AnimeRating::Rx));
        assert_eq!(AnimeRating::parse("Rx - Hentai"), None);
        assert_eq!(AnimeRating::parse("G"), Some(AnimeRating::G));
    }

    #[test]
    fn anime_season_labels_are_lowercase_like_php() {
        assert_eq!(AnimeSeason::Summer.as_str(), "summer");
        assert_eq!(AnimeSeason::Winter.as_str(), "winter");
        assert_eq!(AnimeSeason::parse("FALL"), Some(AnimeSeason::Fall));
    }

    #[test]
    fn anime_order_by_rejects_mal_sort_keys() {
        assert_eq!(
            AnimeOrderBy::parse("start_date"),
            Some(AnimeOrderBy::StartDate)
        );
        assert_eq!(AnimeOrderBy::StartDate.as_str(), "aired.from");
        assert_eq!(AnimeOrderBy::parse("aired.from"), None);
        assert_eq!(AnimeOrderBy::parse("aired.to"), None);
        assert_eq!(AnimeOrderBy::parse("MAL_ID"), Some(AnimeOrderBy::MalId));
    }

    #[test]
    fn schedule_filter_weekday_labels_are_lowercase_and_is_week_day() {
        assert_eq!(AnimeScheduleFilter::Monday.as_str(), "monday");
        assert_eq!(AnimeScheduleFilter::Sunday.as_str(), "sunday");
        assert_eq!(
            AnimeScheduleFilter::Other.as_str(),
            "Not scheduled once per week"
        );
        assert_eq!(AnimeScheduleFilter::Unknown.as_str(), "Unknown");
        assert_eq!(
            AnimeScheduleFilter::parse("MONDAY"),
            Some(AnimeScheduleFilter::Monday)
        );
        assert_eq!(
            AnimeScheduleFilter::parse("Not scheduled once per week"),
            None
        );
        assert!(AnimeScheduleFilter::Monday.is_week_day());
        assert!(AnimeScheduleFilter::Sunday.is_week_day());
        assert!(!AnimeScheduleFilter::Other.is_week_day());
        assert!(!AnimeScheduleFilter::Unknown.is_week_day());
    }

    #[test]
    fn anime_list_status_labels_are_mal_codes_and_input_uses_indexes() {
        assert_eq!(AnimeListStatus::All.as_str(), "7");
        assert_eq!(AnimeListStatus::Watching.as_str(), "1");
        assert_eq!(AnimeListStatus::PlanToWatch.as_str(), "6");
        assert_eq!(
            AnimeListStatus::parse("plantowatch"),
            Some(AnimeListStatus::PlanToWatch)
        );
        // Numeric labels are not accepted.
        assert_eq!(AnimeListStatus::parse("7"), None);
        assert_eq!(AnimeListStatus::parse("1"), None);
    }

    #[test]
    fn airing_status_filter_is_unresolvable_in_php() {
        const { assert!(!AnimeListAiringStatusFilter::RESOLVES_IN_PHP) };
        for index in AnimeListAiringStatusFilter::INDEXES {
            assert_eq!(
                AnimeListAiringStatusFilter::parse(index),
                None,
                "PHP rejects `{index}`"
            );
        }
        assert_eq!(AnimeListAiringStatusFilter::parse("airing"), None);
        assert!("airing".parse::<AnimeListAiringStatusFilter>().is_err());
        assert_eq!(
            AnimeListAiringStatusFilter::PHP_CLASS,
            "App\\Enums\\AnimeListAiringStatusFilterEnum"
        );
    }

    #[test]
    fn user_anime_list_order_by_matches_php() {
        assert_eq!(UserAnimeListOrderBy::Title.as_str(), "1");
        assert_eq!(UserAnimeListOrderBy::StartedDate.as_str(), "3");
        assert_eq!(UserAnimeListOrderBy::EpisodesWatched.as_str(), "12");
        assert_eq!(UserAnimeListOrderBy::Status.as_str(), "16");
        assert_eq!(
            UserAnimeListOrderBy::parse("started_date"),
            Some(UserAnimeListOrderBy::StartedDate)
        );
        assert_eq!(
            UserAnimeListOrderBy::parse("LAST_UPDATED"),
            Some(UserAnimeListOrderBy::LastUpdated)
        );
        // `finished_date` is in labels() but not a docblock method: rejected.
        assert_eq!(UserAnimeListOrderBy::parse("finished_date"), None);
        // Numeric labels are not accepted as input.
        assert_eq!(UserAnimeListOrderBy::parse("2"), None);
    }
}
