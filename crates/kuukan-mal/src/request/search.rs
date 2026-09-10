//! Ports of `Jikan\Request\Search\*`.
//!
//! Exact query strings matter: the golden fixtures record the URL produced by
//! PHP's `http_build_query()` (RFC 1738 encoding, null values skipped). The
//! `genre[]` entries are appended raw after the query, exactly like the PHP
//! request classes do.

use crate::request::{http_build_query, MalRequest, QueryValue, BASE_URL};

/// `Jikan\Request\Search\AnimeSearchRequest`.
#[derive(Debug, Clone)]
pub struct AnimeSearchRequest {
    query: String,
    page: u64,
    char: Option<String>,
    type_: String,
    score: f64,
    status: i64,
    producer: i64,
    rated: i64,
    start_date: [i64; 3],
    end_date: [i64; 3],
    genre: Vec<i64>,
    genre_exclude: bool,
    order_by: Option<i64>,
    sort: Option<i64>,
}

impl AnimeSearchRequest {
    /// `new AnimeSearchRequest($query = null, $page = 1)`.
    ///
    /// PHP throws `BadResponseException('Search with queries require at least
    /// 3 characters')` for queries shorter than 3 characters; in Kuukan that is
    /// a route-layer validation (`q` DTO rule), so the builder accepts it.
    pub fn new(query: Option<&str>, page: u64) -> Self {
        AnimeSearchRequest {
            query: query.unwrap_or("").to_string(),
            page,
            char: None,
            type_: "0".to_string(),
            score: 0.0,
            status: 0,
            producer: 0,
            rated: 0,
            start_date: [0, 0, 0],
            end_date: [0, 0, 0],
            genre: Vec::new(),
            genre_exclude: false,
            order_by: None,
            sort: None,
        }
    }

    /// `setQuery()`.
    pub fn set_query(&mut self, query: Option<&str>) -> &mut Self {
        self.query = query.unwrap_or("").to_string();
        self
    }

    /// `setPage()`.
    pub fn set_page(&mut self, page: u64) -> &mut Self {
        self.page = page;
        self
    }

    /// `setStartsWithChar()` / `setChar()`.
    pub fn set_starts_with_char(&mut self, char: &str) -> &mut Self {
        self.char = Some(char.to_string());
        self
    }

    /// `setType()`.
    pub fn set_type(&mut self, type_: &str) -> &mut Self {
        self.type_ = type_.to_string();
        self
    }

    /// `setScore()`.
    pub fn set_score(&mut self, score: f64) -> &mut Self {
        self.score = score;
        self
    }

    /// `setStatus()`.
    pub fn set_status(&mut self, status: i64) -> &mut Self {
        self.status = status;
        self
    }

    /// `setProducer()`.
    pub fn set_producer(&mut self, producer: i64) -> &mut Self {
        self.producer = producer;
        self
    }

    /// `setRated()`.
    pub fn set_rated(&mut self, rated: i64) -> &mut Self {
        self.rated = rated;
        self
    }

    /// `setStartDate()`.
    pub fn set_start_date(&mut self, day: i64, month: i64, year: i64) -> &mut Self {
        self.start_date = [day, month, year];
        self
    }

    /// `setEndDate()`.
    pub fn set_end_date(&mut self, day: i64, month: i64, year: i64) -> &mut Self {
        self.end_date = [day, month, year];
        self
    }

    /// `setGenre(...$genre)`: `array_unique(array_merge($genre, $this->genre))`.
    pub fn set_genre(&mut self, genre: impl IntoIterator<Item = i64>) -> &mut Self {
        self.genre = array_unique(genre.into_iter().chain(self.genre.iter().copied()));
        self
    }

    /// `setGenreExclude()`.
    pub fn set_genre_exclude(&mut self, exclude: bool) -> &mut Self {
        self.genre_exclude = exclude;
        self
    }

    /// `setOrderBy()`.
    pub fn set_order_by(&mut self, order_by: i64) -> &mut Self {
        self.order_by = Some(order_by);
        self
    }

    /// `setSort()`.
    pub fn set_sort(&mut self, sort: i64) -> &mut Self {
        self.sort = Some(sort);
        self
    }
}

impl MalRequest for AnimeSearchRequest {
    fn path(&self) -> String {
        let query = http_build_query(&[
            ("q", QueryValue::Str(&self.query)),
            (
                "show",
                if self.page != 1 {
                    QueryValue::UInt(50 * (self.page - 1))
                } else {
                    QueryValue::None
                },
            ),
            ("letter", self.char.as_deref().into()),
            ("type", QueryValue::Str(&self.type_)),
            ("score", QueryValue::Float(self.score)),
            ("status", QueryValue::Int(self.status)),
            ("p", QueryValue::Int(self.producer)),
            ("r", QueryValue::Int(self.rated)),
            ("sd", QueryValue::Int(self.start_date[0])),
            ("sm", QueryValue::Int(self.start_date[1])),
            ("sy", QueryValue::Int(self.start_date[2])),
            ("ed", QueryValue::Int(self.end_date[0])),
            ("em", QueryValue::Int(self.end_date[1])),
            ("ey", QueryValue::Int(self.end_date[2])),
            (
                "gx",
                QueryValue::Int(if self.genre_exclude { 1 } else { 0 }),
            ),
            ("o", opt_int(self.order_by)),
            ("w", opt_int(self.sort)),
        ]);
        let mut query = query;
        append_genre(&mut query, &self.genre);
        format!("{BASE_URL}/anime.php?{query}&c[]=a&c[]=b&c[]=c&c[]=f&c[]=d&c[]=e&c[]=g")
    }
}

/// `Jikan\Request\Search\MangaSearchRequest`.
#[derive(Debug, Clone)]
pub struct MangaSearchRequest {
    query: String,
    page: u64,
    char: Option<String>,
    type_: String,
    score: f64,
    status: i64,
    magazine: i64,
    start_date: [i64; 3],
    end_date: [i64; 3],
    genre: Vec<i64>,
    genre_exclude: bool,
    order_by: Option<i64>,
    sort: Option<i64>,
}

impl MangaSearchRequest {
    /// `new MangaSearchRequest($query = null, $page = 1)`.
    pub fn new(query: Option<&str>, page: u64) -> Self {
        MangaSearchRequest {
            query: query.unwrap_or("").to_string(),
            page,
            char: None,
            type_: "0".to_string(),
            score: 0.0,
            status: 0,
            magazine: 0,
            start_date: [0, 0, 0],
            end_date: [0, 0, 0],
            genre: Vec::new(),
            genre_exclude: false,
            order_by: None,
            sort: None,
        }
    }

    /// `setQuery()`.
    pub fn set_query(&mut self, query: Option<&str>) -> &mut Self {
        self.query = query.unwrap_or("").to_string();
        self
    }

    /// `setPage()`.
    pub fn set_page(&mut self, page: u64) -> &mut Self {
        self.page = page;
        self
    }

    /// `setStartsWithChar()` / `setChar()`.
    pub fn set_starts_with_char(&mut self, char: &str) -> &mut Self {
        self.char = Some(char.to_string());
        self
    }

    /// `setType()`.
    pub fn set_type(&mut self, type_: &str) -> &mut Self {
        self.type_ = type_.to_string();
        self
    }

    /// `setScore()`.
    pub fn set_score(&mut self, score: f64) -> &mut Self {
        self.score = score;
        self
    }

    /// `setStatus()`.
    pub fn set_status(&mut self, status: i64) -> &mut Self {
        self.status = status;
        self
    }

    /// `setMagazine()`.
    pub fn set_magazine(&mut self, magazine: i64) -> &mut Self {
        self.magazine = magazine;
        self
    }

    /// `setStartDate()`.
    pub fn set_start_date(&mut self, day: i64, month: i64, year: i64) -> &mut Self {
        self.start_date = [day, month, year];
        self
    }

    /// `setEndDate()`.
    pub fn set_end_date(&mut self, day: i64, month: i64, year: i64) -> &mut Self {
        self.end_date = [day, month, year];
        self
    }

    /// `setGenre(...$genre)`.
    pub fn set_genre(&mut self, genre: impl IntoIterator<Item = i64>) -> &mut Self {
        self.genre = array_unique(genre.into_iter().chain(self.genre.iter().copied()));
        self
    }

    /// `setGenreExclude()`.
    pub fn set_genre_exclude(&mut self, exclude: bool) -> &mut Self {
        self.genre_exclude = exclude;
        self
    }

    /// `setOrderBy()`.
    pub fn set_order_by(&mut self, order_by: i64) -> &mut Self {
        self.order_by = Some(order_by);
        self
    }

    /// `setSort()`.
    pub fn set_sort(&mut self, sort: i64) -> &mut Self {
        self.sort = Some(sort);
        self
    }
}

impl MalRequest for MangaSearchRequest {
    fn path(&self) -> String {
        let mut query = http_build_query(&[
            ("q", QueryValue::Str(&self.query)),
            (
                "show",
                if self.page != 1 {
                    QueryValue::UInt(50 * (self.page - 1))
                } else {
                    QueryValue::None
                },
            ),
            ("letter", self.char.as_deref().into()),
            ("type", QueryValue::Str(&self.type_)),
            ("score", QueryValue::Float(self.score)),
            ("status", QueryValue::Int(self.status)),
            ("mid", QueryValue::Int(self.magazine)),
            ("sd", QueryValue::Int(self.start_date[0])),
            ("sm", QueryValue::Int(self.start_date[1])),
            ("sy", QueryValue::Int(self.start_date[2])),
            ("ed", QueryValue::Int(self.end_date[0])),
            ("em", QueryValue::Int(self.end_date[1])),
            ("ey", QueryValue::Int(self.end_date[2])),
            (
                "gx",
                QueryValue::Int(if self.genre_exclude { 1 } else { 0 }),
            ),
            ("o", opt_int(self.order_by)),
            ("w", opt_int(self.sort)),
        ]);
        append_genre(&mut query, &self.genre);
        format!("{BASE_URL}/manga.php?{query}&c[]=a&c[]=b&c[]=c&c[]=f&c[]=d&c[]=e&c[]=g")
    }
}

/// `Jikan\Request\Search\CharacterSearchRequest`.
#[derive(Debug, Clone)]
pub struct CharacterSearchRequest {
    query: String,
    page: u64,
    char: Option<String>,
}

impl CharacterSearchRequest {
    /// `new CharacterSearchRequest($query = null, $page = 1)`.
    pub fn new(query: Option<&str>, page: u64) -> Self {
        CharacterSearchRequest {
            query: query.unwrap_or("").to_string(),
            page,
            char: None,
        }
    }

    /// `setQuery()`.
    pub fn set_query(&mut self, query: Option<&str>) -> &mut Self {
        self.query = query.unwrap_or("").to_string();
        self
    }

    /// `setPage()`.
    pub fn set_page(&mut self, page: u64) -> &mut Self {
        self.page = page;
        self
    }

    /// `setStartsWithChar()`.
    pub fn set_starts_with_char(&mut self, char: &str) -> &mut Self {
        self.char = Some(char.to_string());
        self
    }
}

impl MalRequest for CharacterSearchRequest {
    fn path(&self) -> String {
        let query = http_build_query(&[
            ("q", QueryValue::Str(&self.query)),
            (
                "show",
                if self.page != 1 {
                    QueryValue::UInt(50 * (self.page - 1))
                } else {
                    QueryValue::None
                },
            ),
            ("letter", self.char.as_deref().into()),
        ]);
        format!("{BASE_URL}/character.php?{query}")
    }
}

/// `Jikan\Request\Search\PersonSearchRequest`.
#[derive(Debug, Clone)]
pub struct PersonSearchRequest {
    query: String,
    page: u64,
    char: Option<String>,
}

impl PersonSearchRequest {
    /// `new PersonSearchRequest($query = null, $page = 1)`.
    pub fn new(query: Option<&str>, page: u64) -> Self {
        PersonSearchRequest {
            query: query.unwrap_or("").to_string(),
            page,
            char: None,
        }
    }

    /// `setQuery()`.
    pub fn set_query(&mut self, query: Option<&str>) -> &mut Self {
        self.query = query.unwrap_or("").to_string();
        self
    }

    /// `setPage()`.
    pub fn set_page(&mut self, page: u64) -> &mut Self {
        self.page = page;
        self
    }

    /// `setStartsWithChar()`.
    pub fn set_starts_with_char(&mut self, char: &str) -> &mut Self {
        self.char = Some(char.to_string());
        self
    }
}

impl MalRequest for PersonSearchRequest {
    fn path(&self) -> String {
        let query = http_build_query(&[
            ("q", QueryValue::Str(&self.query)),
            (
                "show",
                if self.page != 1 {
                    QueryValue::UInt(50 * (self.page - 1))
                } else {
                    QueryValue::None
                },
            ),
            ("letter", self.char.as_deref().into()),
        ]);
        format!("{BASE_URL}/people.php?{query}")
    }
}

/// `Jikan\Request\Search\UserSearchRequest`.
#[derive(Debug, Clone)]
pub struct UserSearchRequest {
    query: Option<String>,
    page: Option<i64>,
    location: Option<String>,
    min_age: Option<i64>,
    max_age: Option<i64>,
    gender: Option<i64>,
}

impl UserSearchRequest {
    /// `new UserSearchRequest($query = null, $page = 1)`.
    pub fn new(query: Option<&str>, page: i64) -> Self {
        UserSearchRequest {
            query: Some(query.unwrap_or("").to_string()),
            page: Some(page),
            location: None,
            min_age: Some(0),
            max_age: Some(0),
            gender: Some(-1), // Constants::SEARCH_USER_GENDER_ANY
        }
    }

    /// `setQuery()`.
    pub fn set_query(&mut self, query: Option<&str>) -> &mut Self {
        self.query = query.map(|q| q.to_string());
        self
    }

    /// `setPage()`.
    pub fn set_page(&mut self, page: Option<i64>) -> &mut Self {
        self.page = page;
        self
    }

    /// `setLocation()`.
    pub fn set_location(&mut self, location: Option<&str>) -> &mut Self {
        self.location = location.map(|l| l.to_string());
        self
    }

    /// `setMinAge()`.
    pub fn set_min_age(&mut self, min_age: Option<i64>) -> &mut Self {
        self.min_age = min_age;
        self
    }

    /// `setMaxAge()`.
    pub fn set_max_age(&mut self, max_age: Option<i64>) -> &mut Self {
        self.max_age = max_age;
        self
    }

    /// `setGender()`.
    pub fn set_gender(&mut self, gender: Option<i64>) -> &mut Self {
        self.gender = gender;
        self
    }
}

impl MalRequest for UserSearchRequest {
    fn path(&self) -> String {
        let query = http_build_query(&[
            ("q", self.query.as_deref().into()),
            // `($this->page !== 1) ? 24 * ($this->page - 1) : null`; a null
            // page still takes the multiplication branch in PHP (`null - 1`
            // is -1), yielding `show=-24`.
            (
                "show",
                match self.page {
                    Some(1) => QueryValue::None,
                    Some(page) => QueryValue::Int(24 * (page - 1)),
                    None => QueryValue::Int(-24),
                },
            ),
            ("loc", self.location.as_deref().into()),
            ("agelow", opt_int(self.min_age)),
            ("agehigh", opt_int(self.max_age)),
            ("g", opt_int(self.gender)),
        ]);
        format!("{BASE_URL}/users.php?{query}")
    }
}

/// PHP `http_build_query()` drops nulls; `None` maps to [`QueryValue::None`].
fn opt_int(value: Option<i64>) -> QueryValue<'static> {
    match value {
        Some(value) => QueryValue::Int(value),
        None => QueryValue::None,
    }
}

/// `array_unique()` preserving the first occurrence, like PHP.
fn array_unique(values: impl IntoIterator<Item = i64>) -> Vec<i64> {
    let mut out = Vec::new();
    for value in values {
        if !out.contains(&value) {
            out.push(value);
        }
    }
    out
}

/// Appends `&genre[]=<id>` for every genre (PHP concatenates the raw id).
fn append_genre(query: &mut String, genres: &[i64]) {
    for genre in genres {
        query.push_str(&format!("&genre[]={genre}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Expected URLs captured from the golden fixtures manifest.
    #[test]
    fn anime_search_matches_recorded_url() {
        let request = AnimeSearchRequest::new(Some("Fate"), 1);
        assert_eq!(
            request.path(),
            "https://myanimelist.net/anime.php?q=Fate&type=0&score=0&status=0&p=0&r=0&sd=0&sm=0&sy=0&ed=0&em=0&ey=0&gx=0&c[]=a&c[]=b&c[]=c&c[]=f&c[]=d&c[]=e&c[]=g"
        );
    }

    #[test]
    fn anime_search_second_page_show_and_letter() {
        let mut request = AnimeSearchRequest::new(Some("Fate"), 2);
        request.set_starts_with_char("A");
        request.set_genre([1, 22]);
        let path = request.path();
        assert!(path.contains("&show=50&letter=A&"), "{path}");
        assert!(path.ends_with("&genre[]=1&genre[]=22&c[]=a&c[]=b&c[]=c&c[]=f&c[]=d&c[]=e&c[]=g"), "{path}");
    }

    #[test]
    fn manga_search_matches_recorded_url() {
        let request = MangaSearchRequest::new(Some("Fate"), 1);
        assert_eq!(
            request.path(),
            "https://myanimelist.net/manga.php?q=Fate&type=0&score=0&status=0&mid=0&sd=0&sm=0&sy=0&ed=0&em=0&ey=0&gx=0&c[]=a&c[]=b&c[]=c&c[]=f&c[]=d&c[]=e&c[]=g"
        );
    }

    #[test]
    fn character_and_person_search_urls() {
        assert_eq!(
            CharacterSearchRequest::new(Some("Testarossa"), 1).path(),
            "https://myanimelist.net/character.php?q=Testarossa"
        );
        assert_eq!(
            PersonSearchRequest::new(Some("Ara"), 1).path(),
            "https://myanimelist.net/people.php?q=Ara"
        );
    }

    #[test]
    fn user_search_url() {
        let mut request = UserSearchRequest::new(Some("neo"), 1);
        request.set_page(Some(2));
        request.set_location(Some("US"));
        request.set_min_age(Some(18));
        request.set_max_age(Some(30));
        request.set_gender(Some(1));
        assert_eq!(
            request.path(),
            "https://myanimelist.net/users.php?q=neo&show=24&loc=US&agelow=18&agehigh=30&g=1"
        );
    }

    #[test]
    fn user_search_null_page_and_gender_match_php() {
        let mut request = UserSearchRequest::new(Some("neo"), 1);
        request.set_page(None);
        request.set_gender(None);
        // PHP: `null !== 1` is true, `24 * (null - 1)` is -24; null params
        // are skipped by http_build_query.
        assert_eq!(
            request.path(),
            "https://myanimelist.net/users.php?q=neo&show=-24&agelow=0&agehigh=0"
        );
    }

    #[test]
    fn genre_dedupe_keeps_first_occurrence() {
        let mut request = AnimeSearchRequest::new(Some("x"), 1);
        request.set_genre([2, 1]);
        request.set_genre([1, 3]);
        let path = request.path();
        assert!(path.ends_with("&genre[]=1&genre[]=3&genre[]=2&c[]=a&c[]=b&c[]=c&c[]=f&c[]=d&c[]=e&c[]=g"), "{path}");
    }
}
