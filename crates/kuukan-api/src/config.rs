//! Runtime configuration.
//!
//! Reads the same environment variables as Jikan (`.env.dist`) plus
//! `KUUKAN_*` settings for storage/search. Defaults match jikan-rest.

use std::path::PathBuf;
use std::time::Duration;

/// Cache TTL categories, mirroring `config/jikan.php::per_endpoint_cache_ttl`
/// collapsed to the env variables that define the values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheCategory {
    Default,
    User,
    UserList,
    Search,
    Magazine,
    Genre,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub app_version: String,
    pub debug: bool,

    // Storage / search
    pub db_path: PathBuf,
    pub search_path: PathBuf,

    // MAL source
    pub source_timeout: Duration,
    pub mal_proxy: Option<String>,
    pub mal_user_agent: Option<String>,

    // Endpoints
    pub max_results_per_page: u64,
    pub disable_user_lists: bool,
    pub insights: bool,

    // Caching
    pub microcaching: bool,
    pub microcaching_expire: u64,
    pub cache_default_ttl: u64,
    pub cache_user_ttl: u64,
    pub cache_userlist_ttl: u64,
    pub cache_search_ttl: u64,
    pub cache_magazine_ttl: u64,
    pub cache_genre_ttl: u64,

    // CORS
    pub cors_middleware: bool,

    // Optional public style rate limiting
    pub rate_limit_enabled: bool,
    pub rate_limit_per_second: u64,
    pub rate_limit_per_minute: u64,
}

fn env_string(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(default)
}

fn env_bool(key: &str, default: bool) -> bool {
    match std::env::var(key) {
        Ok(v) => matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "on" | "yes"
        ),
        Err(_) => default,
    }
}

impl Config {
    pub fn from_env() -> Self {
        Config {
            app_version: env_string("APP_VERSION", "4.2.2"),
            debug: env_bool("APP_DEBUG", false),
            db_path: PathBuf::from(env_string("KUUKAN_DB_PATH", "data/kuukan.db")),
            search_path: PathBuf::from(env_string("KUUKAN_SEARCH_PATH", "data/search")),
            source_timeout: Duration::from_secs(env_u64("SOURCE_TIMEOUT", 10)),
            mal_proxy: std::env::var("KUUKAN_MAL_PROXY")
                .or_else(|_| std::env::var("HTTPS_PROXY"))
                .ok(),
            mal_user_agent: std::env::var("KUUKAN_MAL_USER_AGENT").ok(),
            max_results_per_page: env_u64("MAX_RESULTS_PER_PAGE", 25).max(1),
            disable_user_lists: env_bool("DISABLE_USER_LISTS", false),
            insights: env_bool("INSIGHTS", false),
            microcaching: env_bool("MICROCACHING", false),
            microcaching_expire: env_u64("MICROCACHING_EXPIRE", 5),
            cache_default_ttl: env_u64("CACHE_DEFAULT_EXPIRE", 86_400),
            cache_user_ttl: env_u64("CACHE_USER_EXPIRE", 300),
            cache_userlist_ttl: env_u64("CACHE_USERLIST_EXPIRE", 3_600),
            cache_search_ttl: env_u64("CACHE_SEARCH_EXPIRE", 432_000),
            cache_magazine_ttl: env_u64("CACHE_MAGAZINE_EXPIRE", 432_000),
            cache_genre_ttl: env_u64("CACHE_GENRE_EXPIRE", 432_000),
            cors_middleware: env_bool("CORS_MIDDLEWARE", false),
            rate_limit_enabled: env_bool("KUUKAN_RATE_LIMIT", false),
            rate_limit_per_second: env_u64("KUUKAN_RATE_LIMIT_PER_SECOND", 3),
            rate_limit_per_minute: env_u64("KUUKAN_RATE_LIMIT_PER_MINUTE", 60),
        }
    }

    pub fn cache_ttl(&self, category: CacheCategory) -> u64 {
        match category {
            CacheCategory::Default => self.cache_default_ttl,
            CacheCategory::User => self.cache_user_ttl,
            CacheCategory::UserList => self.cache_userlist_ttl,
            CacheCategory::Search => self.cache_search_ttl,
            CacheCategory::Magazine => self.cache_magazine_ttl,
            CacheCategory::Genre => self.cache_genre_ttl,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            app_version: "4.2.2".to_string(),
            debug: false,
            db_path: PathBuf::from("data/kuukan.db"),
            search_path: PathBuf::from("data/search"),
            source_timeout: Duration::from_secs(10),
            mal_proxy: None,
            mal_user_agent: None,
            max_results_per_page: 25,
            disable_user_lists: false,
            insights: false,
            microcaching: false,
            microcaching_expire: 5,
            cache_default_ttl: 86_400,
            cache_user_ttl: 300,
            cache_userlist_ttl: 3_600,
            cache_search_ttl: 432_000,
            cache_magazine_ttl: 432_000,
            cache_genre_ttl: 432_000,
            cors_middleware: false,
            rate_limit_enabled: false,
            rate_limit_per_second: 3,
            rate_limit_per_minute: 60,
        }
    }
}
