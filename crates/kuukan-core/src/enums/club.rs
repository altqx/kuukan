//! Club enums.

php_enum! {
    /// PHP `App\Enums\ClubTypeEnum` (`type` on `/clubs`).
    pub enum ClubType = "App\\Enums\\ClubTypeEnum" {
        Public => "public" => "public",
        Private => "private" => "private",
        Secret => "secret" => "secret",
    }
}

php_enum! {
    /// PHP `App\Enums\ClubCategoryEnum` (`category` on `/clubs`).
    pub enum ClubCategory = "App\\Enums\\ClubCategoryEnum" {
        Anime => "anime" => "Anime",
        Manga => "manga" => "Manga",
        ActorsAndArtists => "actors_and_artists" => "Actors & Artists",
        Characters => "characters" => "Characters",
        CitiesAndNeighborhoods => "cities_and_neighborhoods" => "Cities & Neighborhoods",
        Companies => "companies" => "Companies",
        Conventions => "conventions" => "Conventions",
        Games => "games" => "Games",
        Japan => "japan" => "Japan",
        Music => "music" => "Music",
        Other => "other" => "Other",
        Schools => "schools" => "Schools",
    }
}

php_enum! {
    /// PHP `App\Enums\ClubOrderByEnum` (`order_by` on `/clubs`).
    ///
    /// `members_count` maps to the MAL sort key `members`; the label is not
    /// accepted as input.
    pub enum ClubOrderBy = "App\\Enums\\ClubOrderByEnum" {
        MalId => "mal_id" => "mal_id",
        Name => "name" => "name",
        MembersCount => "members_count" => "members",
        Created => "created" => "created",
    }
}

/// `App\Enums\ClubTypeEnum::from()` as an `Option`.
pub fn parse_club_type(value: &str) -> Option<ClubType> {
    ClubType::parse(value)
}

/// `App\Enums\ClubCategoryEnum::from()` as an `Option`.
pub fn parse_club_category(value: &str) -> Option<ClubCategory> {
    ClubCategory::parse(value)
}

/// `App\Enums\ClubOrderByEnum::from()` as an `Option`.
pub fn parse_club_order_by(value: &str) -> Option<ClubOrderBy> {
    ClubOrderBy::parse(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enums::testutil::assert_enum_roundtrip;

    #[test]
    fn club_enums_roundtrip() {
        assert_enum_roundtrip(ClubType::ALL, ClubType::INDEXES, ClubType::LABELS);
        assert_enum_roundtrip(
            ClubCategory::ALL,
            ClubCategory::INDEXES,
            ClubCategory::LABELS,
        );
        assert_enum_roundtrip(ClubOrderBy::ALL, ClubOrderBy::INDEXES, ClubOrderBy::LABELS);
    }

    #[test]
    fn club_category_labels_are_not_accepted_as_input() {
        assert_eq!(
            parse_club_category("actors_and_artists"),
            Some(ClubCategory::ActorsAndArtists)
        );
        assert_eq!(ClubCategory::ActorsAndArtists.as_str(), "Actors & Artists");
        assert_eq!(parse_club_category("Actors & Artists"), None);
        assert_eq!(
            parse_club_category("cities_and_neighborhoods"),
            Some(ClubCategory::CitiesAndNeighborhoods)
        );
    }

    #[test]
    fn club_order_by_rejects_mal_sort_key() {
        assert_eq!(
            parse_club_order_by("members_count"),
            Some(ClubOrderBy::MembersCount)
        );
        assert_eq!(ClubOrderBy::MembersCount.as_str(), "members");
        assert_eq!(parse_club_order_by("members"), None);
    }
}
