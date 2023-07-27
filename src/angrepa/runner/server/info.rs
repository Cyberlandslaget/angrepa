use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use serde_json::{json, Value};
use std::sync::Arc;

use super::AppState;

async fn internal_tick(State(state): State<Arc<AppState>>) -> (StatusCode, Json<Value>) {
    let tick = state
        .config
        .common
        .current_tick(chrono::Utc::now())
        .to_string();
    (
        StatusCode::OK,
        json!({ "status": "ok", "tick": tick}).into(),
    )
}

// /logs/
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/internal_tick", get(internal_tick))
        .with_state(state)
}
