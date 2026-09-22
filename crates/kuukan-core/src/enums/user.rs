//! User-list enums.

php_enum! {
    /// PHP `App\Enums\UserListTypeEnum`.
    ///
    /// Internal selector between the anime and manga list mappers; it is not
    /// bound to a request parameter.
    pub enum UserListType = "App\\Enums\\UserListTypeEnum" {
        Anime => "anime" => "anime",
        Manga => "manga" => "manga",
    }
}

php_enum! {
    /// PHP `App\Enums\UserHistoryTypeEnum` (`type` on user history routes).
    pub enum UserHistoryType = "App\\Enums\\UserHistoryTypeEnum" {
        Anime => "anime" => "anime",
        Manga => "manga" => "manga",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enums::testutil::assert_enum_roundtrip;

    #[test]
    fn user_enums_roundtrip() {
        assert_enum_roundtrip(
            UserListType::ALL,
            UserListType::INDEXES,
            UserListType::LABELS,
        );
        assert_enum_roundtrip(
            UserHistoryType::ALL,
            UserHistoryType::INDEXES,
            UserHistoryType::LABELS,
        );
    }

    #[test]
    fn user_types_accept_case_insensitive_indexes() {
        assert_eq!(UserListType::parse("ANIME"), Some(UserListType::Anime));
        assert_eq!(
            UserHistoryType::parse("Manga"),
            Some(UserHistoryType::Manga)
        );
        assert_eq!(UserListType::parse("novel"), None);
    }
}
