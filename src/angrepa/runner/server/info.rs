use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use serde_json::{json, Value};
use std::sync::Arc;

use angrepa::db::Db;

use super::AppState;

async fn internal_tick(State(state): State<Arc<AppState>>) -> (StatusCode, Json<Value>) {
    let tick = state
        .config
        .common
        .current_tick(chrono::Utc::now())
        .to_string();
    (
        StatusCode::OK,
        json!({ "status": "ok", "data": tick}).into(),
    )
}

// GET /info/teams
async fn teams(State(state): State<Arc<AppState>>) -> (StatusCode, Json<Value>) {
    let mut conn = state.db.get().unwrap();
    let mut db = Db::new(&mut conn);

    match db.teams() {
        Ok(teams) => (
            StatusCode::OK,
            json!({ "status": "ok", "data": teams}).into(),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "status": "error", "message": format!("Failed to get teams: {:?}", e) }).into(),
        ),
    }
}

// GET /info/services
async fn services(State(state): State<Arc<AppState>>) -> (StatusCode, Json<Value>) {
    let mut conn = state.db.get().unwrap();
    let mut db = Db::new(&mut conn);

    match db.services() {
        Ok(services) => (
            StatusCode::OK,
            json!({ "status": "ok", "data": services}).into(),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "status": "error", "message": format!("Failed to get services: {:?}", e) })
                .into(),
        ),
    }
}

// /info/
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/internal_tick", get(internal_tick))
        .route("/teams", get(teams))
        .route("/services", get(services))
        .with_state(state)
}
