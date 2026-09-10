//! Response resource mappers.
//!
//! Ported from `app/Http/Resources/V4/*.php`. Resources are pure mappings from
//! stored/cached payload documents (`serde_json::Value`, JMS snake_case shape)
//! to API response JSON. Every mapper must emit all keys the PHP resource
//! emits, including `null` values, and preserve Jikan quirks such as an empty
//! `related` being rendered as `{}`.

pub mod anime;
pub mod character;
pub mod club;
pub mod forum;
pub mod genre;
pub mod magazine;
pub mod manga;
pub mod misc;
pub mod news;
pub mod person;
pub mod producer;
pub mod recommendations;
pub mod reviews;
pub mod schedule;
pub mod search;
pub mod season;
pub mod top;
pub mod user;
pub mod watch;
