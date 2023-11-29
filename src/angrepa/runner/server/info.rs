use axum::{
    extract,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};
use std::sync::Arc;

use super::AppState;

// GET /info/internal_tick
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
    match state.db.teams().await {
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

// GET /info/team/:ip
async fn team(
    State(state): State<Arc<AppState>>,
    Path(ip): Path<String>,
) -> (StatusCode, Json<Value>) {
    match state.db.team_by_ip(&ip).await {
        Ok(Some(team)) => (
            StatusCode::OK,
            json!({ "status": "ok", "data": team}).into(),
        ),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            json!({ "status": "error", "message": "team not found" }).into(),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "status": "error", "message": format!("Failed to get team: {:?}", e) }).into(),
        ),
    }
}

#[derive(serde::Deserialize)]
struct JsonConfig {
    ip: String,
    name: String,
}

// POST /info/team/name
async fn team_set_name(
    State(state): State<Arc<AppState>>,
    extract::Json(ipname): extract::Json<JsonConfig>,
) -> (StatusCode, Json<Value>) {
    match state.db.team_set_name(&ipname.ip, &ipname.name).await {
        Ok(()) => (StatusCode::OK, json!({ "status": "ok"}).into()),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "status": "error", "message": format!("Failed to set team name: {:?}", e) })
                .into(),
        ),
    }
}

// GET /info/services
async fn services(State(state): State<Arc<AppState>>) -> (StatusCode, Json<Value>) {
    match state.db.services().await {
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
        .route("/team/:ip", get(team))
        .route("/team/name", post(team_set_name))
        .route("/services", get(services))
        .with_state(state)
}
