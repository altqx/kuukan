//! Store configuration.
//!
//! Kuukan replaces Jikan's MongoDB + Redis pair with a single SQLite database.
//! [`StoreConfig`] carries the two knobs needed to open it: the database file
//! path and the connection pool size.
//!
//! The path is read from `KUUKAN_DB_PATH` (Kuukan-specific settings are prefixed
//! `KUUKAN_`) and defaults to `data/kuukan.db`. The
//! database file is created if it does not exist; parent directories are
//! created automatically by [`crate::Store::open`].

use std::path::PathBuf;

/// Default SQLite database path, relative to the process working directory.
pub const DEFAULT_DB_PATH: &str = "data/kuukan.db";

/// Default maximum number of pooled SQLite connections.
pub const DEFAULT_MAX_CONNECTIONS: u32 = 5;

/// Configuration for opening a [`crate::Store`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreConfig {
    /// Path of the SQLite database file.
    pub path: PathBuf,
    /// Maximum number of pooled connections. Values below 1 are treated as 1.
    pub max_connections: u32,
}

impl StoreConfig {
    /// Create a config for `path` with the default pool size.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            max_connections: DEFAULT_MAX_CONNECTIONS,
        }
    }

    /// Build a config from the environment, reading `KUUKAN_DB_PATH`.
    ///
    /// Missing, empty or non-unicode values fall back to [`DEFAULT_DB_PATH`].
    pub fn from_env() -> Self {
        let path = std::env::var("KUUKAN_DB_PATH")
            .ok()
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| DEFAULT_DB_PATH.to_string());
        Self {
            path: PathBuf::from(path),
            max_connections: DEFAULT_MAX_CONNECTIONS,
        }
    }

    /// Override the maximum number of pooled connections.
    pub fn with_max_connections(mut self, max_connections: u32) -> Self {
        self.max_connections = max_connections;
        self
    }
}

impl Default for StoreConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

/// Read an `i64` tuning knob from the environment, falling back to `default`.
pub(crate) fn env_i64(name: &str, default: i64) -> i64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<i64>().ok())
        .unwrap_or(default)
}

/// Read an `f64` tuning knob from the environment, falling back to `default`.
pub(crate) fn env_f64(name: &str, default: f64) -> f64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<f64>().ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_uses_default_pool_size() {
        let config = StoreConfig::new("data/kuukan.db");
        assert_eq!(config.path, PathBuf::from("data/kuukan.db"));
        assert_eq!(config.max_connections, DEFAULT_MAX_CONNECTIONS);
    }

    #[test]
    fn with_max_connections_overrides() {
        let config = StoreConfig::new(":memory:").with_max_connections(1);
        assert_eq!(config.max_connections, 1);
    }

    #[test]
    fn env_helpers_fall_back_to_default() {
        // These variables are exclusively read by Kuukan and are not expected to
        // be set in test environments; the assertion also holds if they are not.
        assert_eq!(env_i64("KUUKAN_DEFINITELY_UNSET_INT", 7), 7);
        assert_eq!(env_f64("KUUKAN_DEFINITELY_UNSET_FLOAT", 0.5), 0.5);
    }
}
