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

/// `App\Enums\UserListTypeEnum::from()` as an `Option`.
pub fn parse_user_list_type(value: &str) -> Option<UserListType> {
    UserListType::parse(value)
}

/// `App\Enums\UserHistoryTypeEnum::from()` as an `Option`.
pub fn parse_user_history_type(value: &str) -> Option<UserHistoryType> {
    UserHistoryType::parse(value)
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
        assert_eq!(parse_user_list_type("ANIME"), Some(UserListType::Anime));
        assert_eq!(
            parse_user_history_type("Manga"),
            Some(UserHistoryType::Manga)
        );
        assert_eq!(parse_user_list_type("novel"), None);
    }
}
