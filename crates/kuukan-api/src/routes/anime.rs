//! `/anime` routes — port of `AnimeController` (`app/Features/Anime*Handler.php`).
//!
//! Two storage shapes are used:
//!
//! * **entity lookups** (`ItemLookupHandler`) serve the anime document stored
//!   under `(EntityKind::Anime, mal_id)`: `main`, `full`, `relations`,
//!   `themes`, `external`, `streaming`;
//! * **fingerprint caches** (`RequestHandlerWithScraperCache`) scrape and cache
//!   the sub-resource under the request fingerprint: everything else.
//!
//! Single-object resources are wrapped with `envelope::data`; the list
//! resources (`episodes`, `news`, `userupdates`, `reviews`) already return the
//! `{pagination, data}` envelope.

use axum::extract::{OriginalUri, Path, State};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use kuukan_core::enums::AnimeForumFilter;
use kuukan_core::envelope;
use kuukan_store::EntityKind;
use serde_json::json;

use crate::dto::anime::{
    AnimeCharactersLookupCommand, AnimeEpisodeLookupCommand, AnimeEpisodesLookupCommand,
    AnimeExternalLookupCommand, AnimeForumLookupCommand, AnimeFullLookupCommand,
    AnimeLookupCommand, AnimeMoreInfoLookupCommand, AnimeNewsLookupCommand,
    AnimePicturesLookupCommand, AnimeRecommendationsLookupCommand, AnimeRelationsLookupCommand,
    AnimeReviewsLookupCommand, AnimeStaffLookupCommand, AnimeStatsLookupCommand,
    AnimeStreamingLookupCommand, AnimeThemesLookupCommand, AnimeUserUpdatesLookupCommand,
    AnimeVideosEpisodesLookupCommand, AnimeVideosLookupCommand,
};
use crate::endpoint::Endpoint;
use crate::error::ApiErrorResponse;
use crate::extract::{route_id, RawQuery};
use crate::resources::anime as resource;
use crate::resources::misc;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/anime/{id}", get(main))
        .route("/anime/{id}/full", get(full))
        .route("/anime/{id}/characters", get(characters))
        .route("/anime/{id}/staff", get(staff))
        .route("/anime/{id}/episodes", get(episodes))
        .route("/anime/{id}/episodes/{episodeId}", get(episode))
        .route("/anime/{id}/news", get(news))
        .route("/anime/{id}/forum", get(forum))
        .route("/anime/{id}/videos", get(videos))
        .route("/anime/{id}/videos/episodes", get(videos_episodes))
        .route("/anime/{id}/pictures", get(pictures))
        .route("/anime/{id}/statistics", get(statistics))
        .route("/anime/{id}/moreinfo", get(more_info))
        .route("/anime/{id}/recommendations", get(recommendations))
        .route("/anime/{id}/userupdates", get(user_updates))
        .route("/anime/{id}/reviews", get(reviews))
        .route("/anime/{id}/relations", get(relations))
        .route("/anime/{id}/themes", get(themes))
        .route("/anime/{id}/external", get(external))
        .route("/anime/{id}/streaming", get(streaming))
}

/// Everything this module's endpoints share: the request type they are
/// cached and fingerprinted as, and the TTL category they use.
const ENDPOINT: Endpoint = Endpoint::new("anime");

async fn main(
    State(state): State<AppState>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = AnimeLookupCommand::parse(route_id(&id)?, &query)?;
    let mal = state.mal.clone();
    let id = command.id;
    let cached = ENDPOINT
        .entity(&state, &uri, EntityKind::Anime, id, move || async move {
            kuukan_mal::api::anime::get_anime(&mal, id).await
        })
        .await?;
    // `AnimeResource` reads the Eloquent accessors (`season`, `year`,
    // `broadcast`), which the store keeps in their raw JMS shape.
    let payload = crate::collection::materialize_accessors(&cached.payload);
    let data = envelope::data(resource::anime(&payload));
    Ok(cached.render(data))
}

async fn full(
    State(state): State<AppState>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = AnimeFullLookupCommand::parse(route_id(&id)?, &query)?;
    let mal = state.mal.clone();
    let id = command.id;
    let cached = ENDPOINT
        .entity(&state, &uri, EntityKind::Anime, id, move || async move {
            kuukan_mal::api::anime::get_anime(&mal, id).await
        })
        .await?;
    let payload = crate::collection::materialize_accessors(&cached.payload);
    let data = envelope::data(resource::anime_full(&payload));
    Ok(cached.render(data))
}

async fn characters(
    State(state): State<AppState>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = AnimeCharactersLookupCommand::parse(route_id(&id)?, &query)?;
    let mal = state.mal.clone();
    let cached = ENDPOINT
        .document(&state, &uri, move || async move {
            kuukan_mal::api::anime::get_anime_characters_and_staff(&mal, command.id).await
        })
        .await?;
    let data = envelope::data(resource::anime_characters(&cached.payload));
    Ok(cached.render(data))
}

async fn staff(
    State(state): State<AppState>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = AnimeStaffLookupCommand::parse(route_id(&id)?, &query)?;
    let mal = state.mal.clone();
    let cached = ENDPOINT
        .document(&state, &uri, move || async move {
            kuukan_mal::api::anime::get_anime_characters_and_staff(&mal, command.id).await
        })
        .await?;
    let data = envelope::data(resource::anime_staff(&cached.payload));
    Ok(cached.render(data))
}

async fn episodes(
    State(state): State<AppState>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = AnimeEpisodesLookupCommand::parse(route_id(&id)?, &query)?;
    let mal = state.mal.clone();
    let page = command.page;
    let cached = ENDPOINT
        .document(&state, &uri, move || async move {
            kuukan_mal::api::anime::get_anime_episodes(&mal, command.id, Some(page)).await
        })
        .await?;
    let data = resource::anime_episodes(&cached.payload);
    Ok(cached.render(data))
}

async fn episode(
    State(state): State<AppState>,
    Path((id, episode_id)): Path<(String, String)>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = AnimeEpisodeLookupCommand::parse(route_id(&id)?, route_id(&episode_id)?, &query)?;
    let mal = state.mal.clone();
    let cached = ENDPOINT
        .document(&state, &uri, move || async move {
            kuukan_mal::api::anime::get_anime_episode(&mal, command.id, command.episode_id).await
        })
        .await?;
    let data = envelope::data(resource::anime_episode(&cached.payload));
    Ok(cached.render(data))
}

async fn news(
    State(state): State<AppState>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = AnimeNewsLookupCommand::parse(route_id(&id)?, &query)?;
    let mal = state.mal.clone();
    let page = command.page;
    let cached = ENDPOINT
        .document(&state, &uri, move || async move {
            kuukan_mal::api::anime::get_anime_news(&mal, command.id, Some(page)).await
        })
        .await?;
    let data = resource::anime_news(&cached.payload);
    Ok(cached.render(data))
}

async fn forum(
    State(state): State<AppState>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = AnimeForumLookupCommand::parse(route_id(&id)?, &query)?;
    // `AnimeForumLookupHandler` defaults the topic to `all`.
    let topic = command.filter.unwrap_or(AnimeForumFilter::All).as_str();
    let mal = state.mal.clone();
    let cached = ENDPOINT
        .document(&state, &uri, move || async move {
            kuukan_mal::api::anime::get_anime_forum(&mal, command.id, Some(topic)).await
        })
        .await?;
    let data = envelope::data(resource::anime_forum(&cached.payload));
    Ok(cached.render(data))
}

async fn videos(
    State(state): State<AppState>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = AnimeVideosLookupCommand::parse(route_id(&id)?, &query)?;
    let mal = state.mal.clone();
    let cached = ENDPOINT
        .document(&state, &uri, move || async move {
            kuukan_mal::api::anime::get_anime_videos(&mal, command.id).await
        })
        .await?;
    let data = envelope::data(resource::anime_videos(&cached.payload));
    Ok(cached.render(data))
}

async fn videos_episodes(
    State(state): State<AppState>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = AnimeVideosEpisodesLookupCommand::parse(route_id(&id)?, &query)?;
    let mal = state.mal.clone();
    let page = command.page;
    let cached = ENDPOINT
        .document(&state, &uri, move || async move {
            kuukan_mal::api::anime::get_anime_videos_episodes(&mal, command.id, Some(page)).await
        })
        .await?;
    let data = resource::anime_episodes(&cached.payload);
    Ok(cached.render(data))
}

async fn pictures(
    State(state): State<AppState>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = AnimePicturesLookupCommand::parse(route_id(&id)?, &query)?;
    let mal = state.mal.clone();
    let cached = ENDPOINT
        .document(&state, &uri, move || async move {
            let pictures = kuukan_mal::api::anime::get_anime_pictures(&mal, command.id).await?;
            Ok(json!({ "pictures": pictures }))
        })
        .await?;
    let data = envelope::data(resource::anime_pictures(&cached.payload));
    Ok(cached.render(data))
}

async fn statistics(
    State(state): State<AppState>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = AnimeStatsLookupCommand::parse(route_id(&id)?, &query)?;
    let mal = state.mal.clone();
    let cached = ENDPOINT
        .document(&state, &uri, move || async move {
            kuukan_mal::api::anime::get_anime_stats(&mal, command.id).await
        })
        .await?;
    let data = envelope::data(resource::anime_statistics(&cached.payload));
    Ok(cached.render(data))
}

async fn more_info(
    State(state): State<AppState>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = AnimeMoreInfoLookupCommand::parse(route_id(&id)?, &query)?;
    let mal = state.mal.clone();
    let cached = ENDPOINT
        .document(&state, &uri, move || async move {
            kuukan_mal::api::anime::get_anime_more_info(&mal, command.id).await
        })
        .await?;
    let data = envelope::data(resource::anime_more_info(&cached.payload));
    Ok(cached.render(data))
}

async fn recommendations(
    State(state): State<AppState>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = AnimeRecommendationsLookupCommand::parse(route_id(&id)?, &query)?;
    let mal = state.mal.clone();
    let cached = ENDPOINT
        .document(&state, &uri, move || async move {
            let items = kuukan_mal::api::anime::get_anime_recommendations(&mal, command.id).await?;
            Ok(json!({ "recommendations": items }))
        })
        .await?;
    let data = envelope::data(resource::anime_recommendations(&cached.payload));
    Ok(cached.render(data))
}

async fn user_updates(
    State(state): State<AppState>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = AnimeUserUpdatesLookupCommand::parse(route_id(&id)?, &query)?;
    let mal = state.mal.clone();
    let page = command.page;
    let cached = ENDPOINT
        .document(&state, &uri, move || async move {
            kuukan_mal::api::anime::get_anime_recently_updated_by_users(
                &mal,
                command.id,
                Some(page),
            )
            .await
        })
        .await?;
    // `AnimeUserUpdatesLookupHandler` keeps the default `ResultsResource`.
    let data = misc::results(&cached.payload);
    Ok(cached.render(data))
}

async fn reviews(
    State(state): State<AppState>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = AnimeReviewsLookupCommand::parse(route_id(&id)?, &query)?;
    let params = command.review_request_params();
    let mal = state.mal.clone();
    let cached = ENDPOINT
        .document(&state, &uri, move || async move {
            kuukan_mal::api::anime::get_anime_reviews(
                &mal,
                command.id,
                Some(params.page),
                Some(params.sort.as_str()),
                Some(params.spoilers),
                Some(params.preliminary),
            )
            .await
        })
        .await?;
    let data = resource::anime_reviews(&cached.payload);
    Ok(cached.render(data))
}

async fn relations(
    State(state): State<AppState>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = AnimeRelationsLookupCommand::parse(route_id(&id)?, &query)?;
    let mal = state.mal.clone();
    let id = command.id;
    let cached = ENDPOINT
        .entity(&state, &uri, EntityKind::Anime, id, move || async move {
            kuukan_mal::api::anime::get_anime(&mal, id).await
        })
        .await?;
    let data = envelope::data(resource::anime_relations(&cached.payload));
    Ok(cached.render(data))
}

async fn themes(
    State(state): State<AppState>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = AnimeThemesLookupCommand::parse(route_id(&id)?, &query)?;
    let mal = state.mal.clone();
    let id = command.id;
    let cached = ENDPOINT
        .entity(&state, &uri, EntityKind::Anime, id, move || async move {
            kuukan_mal::api::anime::get_anime(&mal, id).await
        })
        .await?;
    let data = envelope::data(resource::anime_themes(&cached.payload));
    Ok(cached.render(data))
}

async fn external(
    State(state): State<AppState>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = AnimeExternalLookupCommand::parse(route_id(&id)?, &query)?;
    let mal = state.mal.clone();
    let id = command.id;
    let cached = ENDPOINT
        .entity(&state, &uri, EntityKind::Anime, id, move || async move {
            kuukan_mal::api::anime::get_anime(&mal, id).await
        })
        .await?;
    let data = envelope::data(resource::anime_external_links(&cached.payload));
    Ok(cached.render(data))
}

async fn streaming(
    State(state): State<AppState>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = AnimeStreamingLookupCommand::parse(route_id(&id)?, &query)?;
    let mal = state.mal.clone();
    let id = command.id;
    let cached = ENDPOINT
        .entity(&state, &uri, EntityKind::Anime, id, move || async move {
            kuukan_mal::api::anime::get_anime(&mal, id).await
        })
        .await?;
    let data = envelope::data(resource::anime_streaming_links(&cached.payload));
    Ok(cached.render(data))
}
