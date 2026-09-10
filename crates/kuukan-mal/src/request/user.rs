//! URL builders for the user endpoints.
//!
//! Port of `Jikan\Request\User\**` (jikan-php v4.0.12). Every `path()` mirrors
//! the corresponding `getPath()` including the PHP quirks:
//!
//! - `UserAnimeListRequest`/`UserMangaListRequest` send `offset = (page - 1) * 300`
//!   and run every field through `http_build_query`, which drops unset (null)
//!   properties;
//! - `UserFriendsRequest` only appends `?p=` when the page is truthy;
//! - `UserHistoryRequest` interpolates a null type as the empty string
//!   (`/history/{user}/`);
//! - `UserReviewsRequest`/`UserRecommendationsRequest` use `%d`, so a null page
//!   becomes `?p=0`.

use crate::request::{http_build_query, MalRequest, QueryValue, BASE_URL};

/// `QueryValue` for a nullable integer (PHP skips `null` values).
fn opt_int(value: Option<i64>) -> QueryValue<'static> {
    match value {
        Some(value) => QueryValue::Int(value),
        None => QueryValue::None,
    }
}

/// `Constants::USER_ANIME_LIST_ALL` / `Constants::USER_MANGA_LIST_ALL`.
pub const USER_LIST_ALL: i64 = 7;
/// `Constants::USER_LIST_SORT_DESCENDING`.
pub const USER_LIST_SORT_DESCENDING: i64 = 1;
/// `Constants::USER_LIST_SORT_ASCENDING`.
pub const USER_LIST_SORT_ASCENDING: i64 = -1;

/// Page size used by MAL's `load.json` endpoints (`(page - 1) * 300`).
pub const USER_LIST_PAGE_SIZE: i64 = 300;

/// `Jikan\Request\User\UserProfileRequest`.
#[derive(Debug, Clone)]
pub struct UserProfileRequest {
    pub username: String,
}

impl UserProfileRequest {
    pub fn new(username: impl Into<String>) -> Self {
        UserProfileRequest {
            username: username.into(),
        }
    }
}

impl MalRequest for UserProfileRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/profile/{}/", self.username)
    }
}

/// `Jikan\Request\User\UserFriendsRequest`.
#[derive(Debug, Clone)]
pub struct UserFriendsRequest {
    pub username: String,
    /// Page number (starts at 1).
    pub page: i64,
}

impl UserFriendsRequest {
    pub fn new(username: impl Into<String>, page: i64) -> Self {
        UserFriendsRequest {
            username: username.into(),
            page,
        }
    }
}

impl MalRequest for UserFriendsRequest {
    fn path(&self) -> String {
        let query = if self.page != 0 {
            format!("?{}", http_build_query(&[("p", self.page.into())]))
        } else {
            String::new()
        };
        format!("{BASE_URL}/profile/{}/friends{query}", self.username)
    }
}

/// `Jikan\Request\User\UserHistoryRequest`.
///
/// The PHP constructor throws `\InvalidArgumentException` for a type other
/// than `anime`/`manga`; [`UserHistoryRequest::try_new`] returns the same
/// message as an error.
#[derive(Debug, Clone)]
pub struct UserHistoryRequest {
    pub username: String,
    pub r#type: Option<String>,
}

impl UserHistoryRequest {
    /// Mirrors the PHP constructor, including its `InvalidArgumentException`.
    pub fn try_new(username: impl Into<String>, r#type: Option<&str>) -> Result<Self, String> {
        let r#type = match r#type {
            None => None,
            Some("anime") => Some("anime".to_string()),
            Some("manga") => Some("manga".to_string()),
            Some(other) => return Err(format!("Type {other} is not valid")),
        };
        Ok(UserHistoryRequest {
            username: username.into(),
            r#type,
        })
    }
}

impl MalRequest for UserHistoryRequest {
    fn path(&self) -> String {
        format!(
            "{BASE_URL}/history/{}/{}",
            self.username,
            self.r#type.as_deref().unwrap_or("")
        )
    }
}

/// `Jikan\Request\User\UserClubsRequest`.
#[derive(Debug, Clone)]
pub struct UserClubsRequest {
    pub username: String,
}

impl UserClubsRequest {
    pub fn new(username: impl Into<String>) -> Self {
        UserClubsRequest {
            username: username.into(),
        }
    }
}

impl MalRequest for UserClubsRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/profile/{}/clubs", self.username)
    }
}

/// `Jikan\Request\User\UsernameByIdRequest`.
#[derive(Debug, Clone)]
pub struct UsernameByIdRequest {
    pub id: i64,
}

impl UsernameByIdRequest {
    pub fn new(id: i64) -> Self {
        UsernameByIdRequest { id }
    }
}

impl MalRequest for UsernameByIdRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/comments.php?id={}", self.id)
    }
}

/// `Jikan\Request\User\RecentlyOnlineUsersRequest`.
#[derive(Debug, Clone, Default)]
pub struct RecentlyOnlineUsersRequest;

impl RecentlyOnlineUsersRequest {
    pub fn new() -> Self {
        RecentlyOnlineUsersRequest
    }
}

impl MalRequest for RecentlyOnlineUsersRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/users.php")
    }
}

/// `Jikan\Request\User\UserReviewsRequest`.
#[derive(Debug, Clone)]
pub struct UserReviewsRequest {
    pub username: String,
    pub page: Option<i64>,
}

impl UserReviewsRequest {
    pub fn new(username: impl Into<String>, page: Option<i64>) -> Self {
        UserReviewsRequest {
            username: username.into(),
            page,
        }
    }
}

impl MalRequest for UserReviewsRequest {
    fn path(&self) -> String {
        format!(
            "{BASE_URL}/profile/{}/reviews?p={}",
            self.username,
            self.page.unwrap_or(0)
        )
    }
}

/// `Jikan\Request\User\UserRecommendationsRequest`.
#[derive(Debug, Clone)]
pub struct UserRecommendationsRequest {
    pub username: String,
    pub page: Option<i64>,
}

impl UserRecommendationsRequest {
    pub fn new(username: impl Into<String>, page: Option<i64>) -> Self {
        UserRecommendationsRequest {
            username: username.into(),
            page,
        }
    }
}

impl MalRequest for UserRecommendationsRequest {
    fn path(&self) -> String {
        format!(
            "{BASE_URL}/profile/{}/recommendations?p={}",
            self.username,
            self.page.unwrap_or(0)
        )
    }
}

/// `Jikan\Request\User\UserAnimeListRequest`.
///
/// `setOrderBy`/`setOrderBy2` in PHP multiply by a sort direction (default
/// [`USER_LIST_SORT_DESCENDING`]); callers that need ascending pass
/// [`USER_LIST_SORT_ASCENDING`] to [`UserAnimeListRequest::set_order_by_sorted`].
#[derive(Debug, Clone)]
pub struct UserAnimeListRequest {
    pub username: String,
    /// 1-based page number; the request sends `(page - 1) * 300` as `offset`.
    pub page: i64,
    pub status: i64,
    pub order_by: Option<i64>,
    pub order_by2: Option<i64>,
    pub title: Option<String>,
    pub season: Option<String>,
    pub season_year: Option<i64>,
    pub aired_from: [Option<i64>; 3],
    pub aired_to: [Option<i64>; 3],
    pub airing_status: Option<i64>,
    pub producer: Option<i64>,
}

impl UserAnimeListRequest {
    /// `new UserAnimeListRequest($username, $page = 1, $status = Constants::USER_ANIME_LIST_ALL)`.
    pub fn new(username: impl Into<String>, page: i64, status: i64) -> Self {
        UserAnimeListRequest {
            username: username.into(),
            page,
            status,
            order_by: None,
            order_by2: None,
            title: None,
            season: None,
            season_year: None,
            aired_from: [None, None, None],
            aired_to: [None, None, None],
            airing_status: None,
            producer: None,
        }
    }

    /// The `offset` query value (`getPage()` in PHP).
    pub fn offset(&self) -> i64 {
        (self.page - 1) * USER_LIST_PAGE_SIZE
    }

    pub fn set_page(&mut self, page: i64) {
        self.page = page;
    }

    pub fn set_status(&mut self, status: i64) {
        self.status = status;
    }

    /// `setOrderBy($orderBy)` with the default descending sort.
    pub fn set_order_by(&mut self, order_by: i64) {
        self.order_by = Some(USER_LIST_SORT_DESCENDING * order_by);
    }

    /// `setOrderBy($orderBy, $sort)`.
    pub fn set_order_by_sorted(&mut self, order_by: i64, sort: i64) {
        self.order_by = Some(sort * order_by);
    }

    /// `setOrderBy2($orderBy2)` with the default descending sort.
    pub fn set_order_by2(&mut self, order_by2: i64) {
        self.order_by2 = Some(USER_LIST_SORT_DESCENDING * order_by2);
    }

    /// `setOrderBy2($orderBy2, $sort)`.
    pub fn set_order_by2_sorted(&mut self, order_by2: i64, sort: i64) {
        self.order_by2 = Some(sort * order_by2);
    }

    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = Some(title.into());
    }

    pub fn set_season(&mut self, season: impl Into<String>) {
        self.season = Some(season.into());
    }

    pub fn set_season_year(&mut self, season_year: i64) {
        self.season_year = Some(season_year);
    }

    /// `setAiredFrom($year, $month = null, $day = null)`.
    pub fn set_aired_from(&mut self, year: i64, month: Option<i64>, day: Option<i64>) {
        self.aired_from = [Some(year), month, day];
    }

    /// `setAiredTo($year, $month = null, $day = null)`.
    pub fn set_aired_to(&mut self, year: i64, month: Option<i64>, day: Option<i64>) {
        self.aired_to = [Some(year), month, day];
    }

    pub fn set_airing_status(&mut self, airing_status: i64) {
        self.airing_status = Some(airing_status);
    }

    pub fn set_producer(&mut self, producer: i64) {
        self.producer = Some(producer);
    }
}

impl MalRequest for UserAnimeListRequest {
    fn path(&self) -> String {
        let query = http_build_query(&[
            ("offset", self.offset().into()),
            ("status", self.status.into()),
            ("order", opt_int(self.order_by)),
            ("order2", opt_int(self.order_by2)),
            ("s", self.title.as_deref().into()),
            ("season", self.season.as_deref().into()),
            ("season_year", opt_int(self.season_year)),
            ("aired_from_year", opt_int(self.aired_from[0])),
            ("aired_from_month", opt_int(self.aired_from[1])),
            ("aired_from_day", opt_int(self.aired_from[2])),
            ("aired_to_year", opt_int(self.aired_to[0])),
            ("aired_to_month", opt_int(self.aired_to[1])),
            ("aired_to_day", opt_int(self.aired_to[2])),
            ("producer", opt_int(self.producer)),
            ("airing_status", opt_int(self.airing_status)),
        ]);
        format!("{BASE_URL}/animelist/{}/load.json?{query}", self.username)
    }
}

/// `Jikan\Request\User\UserMangaListRequest`.
#[derive(Debug, Clone)]
pub struct UserMangaListRequest {
    pub username: String,
    /// 1-based page number; the request sends `(page - 1) * 300` as `offset`.
    pub page: i64,
    pub status: i64,
    pub order_by: Option<i64>,
    pub order_by2: Option<i64>,
    pub title: Option<String>,
    pub published_from: [Option<i64>; 3],
    pub published_to: [Option<i64>; 3],
    pub publishing_status: Option<i64>,
    pub magazine: Option<i64>,
}

impl UserMangaListRequest {
    /// `new UserMangaListRequest($username, $page = 1, $status = 7)`.
    pub fn new(username: impl Into<String>, page: i64, status: i64) -> Self {
        UserMangaListRequest {
            username: username.into(),
            page,
            status,
            order_by: None,
            order_by2: None,
            title: None,
            published_from: [None, None, None],
            published_to: [None, None, None],
            publishing_status: None,
            magazine: None,
        }
    }

    /// The `offset` query value (`getPage()` in PHP).
    pub fn offset(&self) -> i64 {
        (self.page - 1) * USER_LIST_PAGE_SIZE
    }

    pub fn set_page(&mut self, page: i64) {
        self.page = page;
    }

    pub fn set_status(&mut self, status: i64) {
        self.status = status;
    }

    /// `setOrderBy($orderBy)` with the default descending sort.
    pub fn set_order_by(&mut self, order_by: i64) {
        self.order_by = Some(USER_LIST_SORT_DESCENDING * order_by);
    }

    /// `setOrderBy($orderBy, $sort)`.
    pub fn set_order_by_sorted(&mut self, order_by: i64, sort: i64) {
        self.order_by = Some(sort * order_by);
    }

    /// `setOrderBy2($orderBy2)` with the default descending sort.
    pub fn set_order_by2(&mut self, order_by2: i64) {
        self.order_by2 = Some(USER_LIST_SORT_DESCENDING * order_by2);
    }

    /// `setOrderBy2($orderBy2, $sort)`.
    pub fn set_order_by2_sorted(&mut self, order_by2: i64, sort: i64) {
        self.order_by2 = Some(sort * order_by2);
    }

    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = Some(title.into());
    }

    /// `setPublishedFrom($year, $month = null, $day = null)`.
    pub fn set_published_from(&mut self, year: i64, month: Option<i64>, day: Option<i64>) {
        self.published_from = [Some(year), month, day];
    }

    /// `setPublishedTo($year, $month = null, $day = null)`.
    pub fn set_published_to(&mut self, year: i64, month: Option<i64>, day: Option<i64>) {
        self.published_to = [Some(year), month, day];
    }

    pub fn set_publishing_status(&mut self, publishing_status: i64) {
        self.publishing_status = Some(publishing_status);
    }

    pub fn set_magazine(&mut self, magazine: i64) {
        self.magazine = Some(magazine);
    }
}

impl MalRequest for UserMangaListRequest {
    fn path(&self) -> String {
        let query = http_build_query(&[
            ("offset", self.offset().into()),
            ("status", self.status.into()),
            ("order", opt_int(self.order_by)),
            ("order2", opt_int(self.order_by2)),
            ("s", self.title.as_deref().into()),
            ("published_from_year", opt_int(self.published_from[0])),
            ("published_from_month", opt_int(self.published_from[1])),
            ("published_from_day", opt_int(self.published_from[2])),
            ("published_to_year", opt_int(self.published_to[0])),
            ("published_to_month", opt_int(self.published_to[1])),
            ("published_to_day", opt_int(self.published_to[2])),
            ("magazine", opt_int(self.magazine)),
            ("publishing_status", opt_int(self.publishing_status)),
        ]);
        format!("{BASE_URL}/mangalist/{}/load.json?{query}", self.username)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_path() {
        assert_eq!(
            UserProfileRequest::new("sandshark").path(),
            "https://myanimelist.net/profile/sandshark/"
        );
    }

    #[test]
    fn friends_path() {
        assert_eq!(
            UserFriendsRequest::new("morshuwarrior", 1).path(),
            "https://myanimelist.net/profile/morshuwarrior/friends?p=1"
        );
        assert_eq!(
            UserFriendsRequest::new("morshuwarrior", 0).path(),
            "https://myanimelist.net/profile/morshuwarrior/friends"
        );
    }

    #[test]
    fn history_path() {
        assert_eq!(
            UserHistoryRequest::try_new("morshuwarrior", None)
                .unwrap()
                .path(),
            "https://myanimelist.net/history/morshuwarrior/"
        );
        assert_eq!(
            UserHistoryRequest::try_new("morshuwarrior", Some("anime"))
                .unwrap()
                .path(),
            "https://myanimelist.net/history/morshuwarrior/anime"
        );
        assert!(UserHistoryRequest::try_new("morshuwarrior", Some("video")).is_err());
    }

    #[test]
    fn clubs_and_username_by_id_paths() {
        assert_eq!(
            UserClubsRequest::new("sandshark").path(),
            "https://myanimelist.net/profile/sandshark/clubs"
        );
        assert_eq!(
            UsernameByIdRequest::new(3600201).path(),
            "https://myanimelist.net/comments.php?id=3600201"
        );
        assert_eq!(
            RecentlyOnlineUsersRequest::new().path(),
            "https://myanimelist.net/users.php"
        );
    }

    #[test]
    fn reviews_and_recommendations_paths() {
        assert_eq!(
            UserReviewsRequest::new("sandshark", Some(2)).path(),
            "https://myanimelist.net/profile/sandshark/reviews?p=2"
        );
        assert_eq!(
            UserReviewsRequest::new("sandshark", None).path(),
            "https://myanimelist.net/profile/sandshark/reviews?p=0"
        );
        assert_eq!(
            UserRecommendationsRequest::new("sandshark", Some(1)).path(),
            "https://myanimelist.net/profile/sandshark/recommendations?p=1"
        );
    }

    #[test]
    fn anime_list_path_defaults() {
        let request = UserAnimeListRequest::new("morshuwarrior", 1, USER_LIST_ALL);
        assert_eq!(
            request.path(),
            "https://myanimelist.net/animelist/morshuwarrior/load.json?offset=0&status=7"
        );
    }

    #[test]
    fn anime_list_path_second_page_and_filters() {
        let mut request = UserAnimeListRequest::new("morshuwarrior", 2, 1);
        request.set_order_by(4);
        request.set_title("a b");
        request.set_aired_from(2010, Some(1), None);
        request.set_airing_status(1);
        assert_eq!(
            request.path(),
            "https://myanimelist.net/animelist/morshuwarrior/load.json?offset=300&status=1&order=4&s=a+b&aired_from_year=2010&aired_from_month=1&airing_status=1"
        );
    }

    #[test]
    fn manga_list_path_defaults() {
        let request = UserMangaListRequest::new("morshuwarrior", 1, USER_LIST_ALL);
        assert_eq!(
            request.path(),
            "https://myanimelist.net/mangalist/morshuwarrior/load.json?offset=0&status=7"
        );
    }

    #[test]
    fn manga_list_path_filters() {
        let mut request = UserMangaListRequest::new("morshuwarrior", 3, 2);
        request.set_order_by_sorted(1, USER_LIST_SORT_ASCENDING);
        request.set_magazine(5);
        request.set_publishing_status(1);
        assert_eq!(
            request.path(),
            "https://myanimelist.net/mangalist/morshuwarrior/load.json?offset=600&status=2&order=-1&magazine=5&publishing_status=1"
        );
    }
}
