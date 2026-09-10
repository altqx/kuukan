//! Insights routes — `/insights` and `/insights/trends` (config-gated).
//!
//! Port of `InsightsController` + `app/Http/Middleware/Insights.php`. PHP
//! stores one row per *request* in its `insights` table and returns
//! `{pagination: {last_visible_page, has_next_page}, data: [{timestamp, url}]}`
//! (or `[{url, count}]` for trends).
//!
//! In kuukan the metrics table is the search log (`Store::record_search` /
//! `recent_searches`), so each row renders as `{timestamp: created_at, url}`
//! where `url` comes from the metric's `meta.url` when present and the query
//! term otherwise. `/insights/trends` aggregates those rows by their `url`.
//!
//! While `INSIGHTS` is disabled (the default, and the recorded reference
//! environment) both endpoints return the exact PHP 403
//! `InsightsRuntimeException` body.

use std::collections::BTreeMap;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use kuukan_core::error::ApiError;
use kuukan_core::pagination::Pagination;
use kuukan_store::SearchMetric;
use serde_json::{json, Value};

use crate::error::{json_ok, ApiErrorResponse};
use crate::extract::RawQuery;
use crate::state::AppState;

/// `InsightsController::TRENDS`.
const TRENDS: [&str; 4] = ["anime", "manga", "people", "characters"];

/// PHP `env('INSIGHTS_MAX_STORE_TIME', 172800)`.
const DEFAULT_MAX_STORE_TIME: i64 = 172_800;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/insights", get(main))
        .route("/insights/trends", get(trends))
        .layer(axum::middleware::from_fn(super::search::no_cache_default))
}

async fn main(
    State(state): State<AppState>,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    if !state.config.insights {
        return Ok(disabled());
    }

    let (page, limit) = page_limit(&state, &query);
    let window = page.saturating_mul(limit);
    let fetch = window.saturating_add(1).min(i64::MAX as u64) as i64;
    let metrics = state
        .store
        .recent_searches(fetch)
        .await
        .map_err(storage_error)?;

    let cutoff = kuukan_store::now_unix() - max_store_time();
    let alive: Vec<&SearchMetric> = metrics
        .iter()
        .filter(|metric| metric.created_at > cutoff)
        .collect();
    // `window + 1` rows reveal whether anything beyond the requested page
    // matches the time window.
    let has_more = alive.len() as u64 > window
        || (metrics.len() as u64 > window
            && metrics[window as usize].created_at > cutoff);

    let start = page.saturating_sub(1).saturating_mul(limit) as usize;
    let items: Vec<Value> = alive
        .iter()
        .skip(start)
        .take(limit as usize)
        .map(|metric| insight_item(metric))
        .collect();
    let pagination = list_page(page, has_more, items.len() as u64);

    Ok(json_ok(kuukan_core::envelope::paged(&pagination, items)))
}

async fn trends(
    State(state): State<AppState>,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    if !state.config.insights {
        return Ok(disabled());
    }

    let trend = query.get("trend");
    if !trend.is_some_and(|value| TRENDS.contains(&value)) {
        return Err(ApiErrorResponse(ApiError::bad_request(
            "Trend value is invalid",
        )));
    }

    let page = query_page(&query);
    // PHP ignores the request `limit` for the aggregation and always pages with
    // `MAX_RESULTS_PER_PAGE` (`InsightsController::trends`).
    let per_page = state.config.max_results_per_page.max(1);

    let cutoff = kuukan_store::now_unix() - max_store_time();
    let metrics = state
        .store
        .recent_searches(i64::MAX)
        .await
        .map_err(storage_error)?;

    let mut counts: BTreeMap<String, u64> = BTreeMap::new();
    for metric in metrics
        .iter()
        .filter(|metric| metric.created_at > cutoff)
    {
        *counts.entry(metric_url(metric)).or_default() += 1;
    }
    let mut aggregated: Vec<(String, u64)> = counts.into_iter().collect();
    aggregated.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));

    let total = aggregated.len() as u64;
    let last_page = total.div_ceil(per_page).max(1);
    let start = page.saturating_sub(1).saturating_mul(per_page) as usize;
    let items: Vec<Value> = aggregated
        .iter()
        .skip(start)
        .take(per_page as usize)
        .map(|(url, count)| json!({"url": url, "count": count}))
        .collect();
    let pagination = Pagination::list(last_page, page < last_page);

    Ok(json_ok(kuukan_core::envelope::paged(&pagination, items)))
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// PHP `env('INSIGHTS')` false branch.
fn disabled() -> Response {
    let mut response = axum::Json(json!({
        "status": 403,
        "type": "InsightsRuntimeException",
        "message": "Insights service is disabled",
        "error": null,
    }))
    .into_response();
    *response.status_mut() = StatusCode::FORBIDDEN;
    response
}

/// `page`/`limit` parsing: `(int)` cast, `page >= 1`, `1 <= limit <= MAX`.
fn page_limit(state: &AppState, query: &kuukan_core::params::Query) -> (u64, u64) {
    let max = state.config.max_results_per_page.max(1);
    let page = query_page(query);
    let limit = match query.get("limit") {
        None => max,
        Some(raw) => raw
            .trim()
            .parse::<i64>()
            .unwrap_or(0)
            .clamp(1, max as i64) as u64,
    };
    (page, limit)
}

fn query_page(query: &kuukan_core::params::Query) -> u64 {
    query
        .get("page")
        .and_then(|raw| raw.trim().parse::<i64>().ok())
        .unwrap_or(1)
        .max(1) as u64
}

fn max_store_time() -> i64 {
    std::env::var("INSIGHTS_MAX_STORE_TIME")
        .ok()
        .and_then(|value| value.trim().parse::<i64>().ok())
        .unwrap_or(DEFAULT_MAX_STORE_TIME)
}

/// `{timestamp, url}` (`InsightsResource`).
fn insight_item(metric: &SearchMetric) -> Value {
    json!({
        "timestamp": metric.created_at,
        "url": metric_url(metric),
    })
}

/// Kuukan's metrics have no request URL; callers may record one in `meta.url`,
/// otherwise the search term identifies the row (trends groups by it).
fn metric_url(metric: &SearchMetric) -> String {
    metric
        .meta
        .as_ref()
        .and_then(|meta| meta.get("url"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| metric.query.clone())
}

/// List pagination for a fetched window; `has_more` is an exact lower bound,
/// so the last page is only known when this page is the last one.
fn list_page(page: u64, has_more: bool, count: u64) -> Pagination {
    if count == 0 && !has_more {
        return Pagination::list(1, false);
    }
    let last_visible_page = if has_more { page.saturating_add(1) } else { page };
    Pagination::list(last_visible_page, has_more)
}

/// Same rendering as `services::scrape` for storage failures.
fn storage_error(error: kuukan_store::StoreError) -> ApiErrorResponse {
    ApiErrorResponse(ApiError::Storage {
        error: Some(error.to_string()),
        report_url: None,
    })
}
