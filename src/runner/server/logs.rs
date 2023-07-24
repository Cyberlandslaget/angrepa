use axum::{
    body::{Bytes, StreamBody},
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};
use std::sync::Arc;

use angrepa::db::Db;

use super::AppState;

// GET /logs/exploits
async fn exploits_all(State(state): State<Arc<AppState>>) -> (StatusCode, Json<Value>) {
    let mut conn = state.db.get().unwrap();
    let mut db = Db::new(&mut conn);

    match db.exploits_all() {
        Ok(exp) => (StatusCode::OK, json!({ "status": "ok", "data": exp}).into()),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "status": "error", "message": format!("Failed to get exploits: {:?}", e) })
                .into(),
        ),
    }
}

// /logs/
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/exploits", get(exploits_all))
        .with_state(state)
}
