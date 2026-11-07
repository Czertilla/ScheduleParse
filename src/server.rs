use axum::{
    Router,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use tower_http::trace::TraceLayer;

use crate::state::SharedState;

pub fn build_router(calendar_path: &str, state: SharedState) -> Router {
    Router::new()
        .route(calendar_path, get(serve_calendar))
        .route("/healthz", get(health))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn serve_calendar(State(state): State<SharedState>) -> Response {
    let snapshot = state.read().await;
    if snapshot.generated_at.is_none() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "calendar has not been generated yet, try again shortly",
        )
            .into_response();
    }

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/calendar; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "inline; filename=\"calendar.ics\"",
            ),
        ],
        snapshot.ics.clone(),
    )
        .into_response()
}

async fn health(State(state): State<SharedState>) -> Response {
    let snapshot = state.read().await;
    let body = serde_json::json!({
        "generated_at": snapshot.generated_at.map(|dt| dt.to_rfc3339()),
        "event_count": snapshot.event_count,
        "last_error": snapshot.last_error,
    });
    (StatusCode::OK, axum::Json(body)).into_response()
}
