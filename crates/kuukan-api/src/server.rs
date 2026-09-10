//! HTTP server assembly: router, root endpoint and deprecated version stubs.
//!
//! Mirrors `bootstrap/app.php`: the metadata document is served at `/` and
//! `/v4` (the v4 route group contains a `/` route), and `/v1`..`/v3` return
//! the discontinued response.

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use kuukan_core::error::ApiError;
use std::net::SocketAddr;

use crate::error::ApiErrorResponse;
use crate::middleware::{
    cors_layer, default_cache_control_middleware, microcache_middleware, rate_limit_middleware,
    source_health_middleware, MicroCache, RateLimiter,
};
use crate::state::AppState;

fn env_string(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// `/v1` metadata document (the legacy version stubs are intentionally gone:
/// kuukan only speaks v1).
async fn root(State(state): State<AppState>) -> Response {
    let health = state.store.get_health().await.ok();
    let (status, score, down, last_downtime) = match health {
        Some(h) => (
            h.status().to_string(),
            h.score(),
            h.failover,
            h.last_downtime,
        ),
        None => ("HEALTHY".to_string(), 1.0, false, 0),
    };

    Json(serde_json::json!({
        "author_url": "https://github.com/irfan-dahir",
        "discord_url": "http://discord.jikan.moe",
        "version": env_string("APP_VERSION", "4.2.2"),
        "parser_version": env_string("KUUKAN_PARSER_VERSION", "v4.0.12"),
        "website_url": "https://jikan.moe",
        "documentation_url": "https://docs.api.jikan.moe/",
        "github_url": env_string("KUUKAN_GITHUB_URL", "https://github.com/jikan-me/jikan-rest"),
        "parser_github_url": "https://github.com/jikan-me/jikan",
        "production_api_url": env_string("KUUKAN_PUBLIC_API_URL", "/v1/"),
        "status_url": "https://status.jikan.moe",
        "myanimelist_heartbeat": {
            "status": status,
            "score": score,
            "down": down,
            "last_downtime": last_downtime,
        }
    }))
    .into_response()
}

/// Fallback for unknown paths: Jikan renders `HttpException` 404.
async fn not_found() -> Response {
    ApiErrorResponse(ApiError::not_found()).into_response()
}

async fn method_not_allowed() -> Response {
    ApiErrorResponse(ApiError::Http { status: 405 }).into_response()
}

/// Assemble the complete router. Route modules are merged here as they land.
pub fn build_router(state: AppState) -> Router {
    let micro = MicroCache::new(state.config.microcaching, state.config.microcaching_expire);
    let limiter = RateLimiter::new(
        state.config.rate_limit_enabled,
        state.config.rate_limit_per_second,
        state.config.rate_limit_per_minute,
    );

    let api = crate::routes::api_router()
        .route("/", get(root))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            source_health_middleware,
        ));
    let mut router = Router::new()
        .route("/", get(root))
        .nest("/v1", api)
        .fallback(not_found)
        .method_not_allowed_fallback(method_not_allowed);

    if state.config.cors_middleware {
        router = router.layer(cors_layer());
    }
    router = router.layer(axum::middleware::from_fn_with_state(
        micro,
        microcache_middleware,
    ));
    router = router.layer(axum::middleware::from_fn_with_state(
        limiter,
        rate_limit_middleware,
    ));
    // Outermost: ensures every response without configured caching carries
    // Symfony's `Cache-Control: no-cache, private`.
    router = router.layer(axum::middleware::from_fn(default_cache_control_middleware));

    router.with_state(state)
}

/// Run the server until Ctrl-C.
pub async fn serve(listen: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let state = AppState::from_env().await?;
    let app = build_router(state);
    let addr: SocketAddr = listen.parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("kuukan listening on http://{addr}");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutting down");
}
