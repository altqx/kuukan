//! API route assembly.
//!
//! Every route module exposes `pub fn router() -> Router<AppState>`; this
//! module merges them and `server.rs` mounts the result at `/v1`.

use axum::Router;

use crate::state::AppState;

pub mod anime;
pub mod character;
pub mod club;
pub mod genre;
pub mod insights;
pub mod magazine;
pub mod manga;
pub mod misc;
pub mod person;
pub mod producer;
pub mod random;
pub mod recommendations;
pub mod reviews;
pub mod schedule;
pub mod search;
pub mod season;
pub mod top;
pub mod user;
pub mod watch;

/// All API routers, merged.
pub fn api_router() -> Router<AppState> {
    Router::new()
        .merge(genre::router())
        .merge(producer::router())
        .merge(anime::router())
        .merge(manga::router())
        .merge(character::router())
        .merge(person::router())
        .merge(club::router())
        .merge(season::router())
        .merge(schedule::router())
        .merge(watch::router())
        .merge(reviews::router())
        .merge(recommendations::router())
        .merge(user::router())
        .merge(search::router())
        .merge(top::router())
        .merge(random::router())
        .merge(insights::router())
}
