//! Search `order_by` enums for characters, people, producers and magazines.

php_enum! {
    /// PHP `App\Enums\CharacterOrderByEnum` (`order_by` on `/characters`).
    ///
    /// `favorites` maps to the MAL sort key `member_favorites`; the label is
    /// not accepted as input.
    pub enum CharacterOrderBy = "App\\Enums\\CharacterOrderByEnum" {
        MalId => "mal_id" => "mal_id",
        Name => "name" => "name",
        Favorites => "favorites" => "member_favorites",
    }
}

php_enum! {
    /// PHP `App\Enums\PeopleOrderByEnum` (`order_by` on `/people`).
    ///
    /// `favorites` maps to the MAL sort key `member_favorites`.
    pub enum PeopleOrderBy = "App\\Enums\\PeopleOrderByEnum" {
        MalId => "mal_id" => "mal_id",
        Name => "name" => "name",
        Birthday => "birthday" => "birthday",
        Favorites => "favorites" => "member_favorites",
    }
}

php_enum! {
    /// PHP `App\Enums\ProducerOrderByEnum` (`order_by` on `/producers`).
    pub enum ProducerOrderBy = "App\\Enums\\ProducerOrderByEnum" {
        MalId => "mal_id" => "mal_id",
        Count => "count" => "count",
        Favorites => "favorites" => "favorites",
        Established => "established" => "established",
    }
}

php_enum! {
    /// PHP `App\Enums\MagazineOrderByEnum` (`order_by` on `/magazines`).
    pub enum MagazineOrderBy = "App\\Enums\\MagazineOrderByEnum" {
        MalId => "mal_id" => "mal_id",
        Name => "name" => "name",
        Count => "count" => "count",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enums::testutil::assert_enum_roundtrip;

    #[test]
    fn search_enums_roundtrip() {
        assert_enum_roundtrip(
            CharacterOrderBy::ALL,
            CharacterOrderBy::INDEXES,
            CharacterOrderBy::LABELS,
        );
        assert_enum_roundtrip(
            PeopleOrderBy::ALL,
            PeopleOrderBy::INDEXES,
            PeopleOrderBy::LABELS,
        );
        assert_enum_roundtrip(
            ProducerOrderBy::ALL,
            ProducerOrderBy::INDEXES,
            ProducerOrderBy::LABELS,
        );
        assert_enum_roundtrip(
            MagazineOrderBy::ALL,
            MagazineOrderBy::INDEXES,
            MagazineOrderBy::LABELS,
        );
    }

    #[test]
    fn favorites_maps_to_member_favorites_label_only() {
        assert_eq!(CharacterOrderBy::Favorites.as_str(), "member_favorites");
        assert_eq!(PeopleOrderBy::Favorites.as_str(), "member_favorites");
        assert_eq!(
            CharacterOrderBy::parse("favorites"),
            Some(CharacterOrderBy::Favorites)
        );
        assert_eq!(CharacterOrderBy::parse("member_favorites"), None);
        assert_eq!(PeopleOrderBy::parse("member_favorites"), None);
        assert_eq!(
            PeopleOrderBy::parse("birthday"),
            Some(PeopleOrderBy::Birthday)
        );
        assert_eq!(
            ProducerOrderBy::parse("established"),
            Some(ProducerOrderBy::Established)
        );
        assert_eq!(
            MagazineOrderBy::parse("count"),
            Some(MagazineOrderBy::Count)
        );
    }
}
