use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use chrono::NaiveDateTime;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use angrepa::db::Db;

use super::AppState;

#[derive(Deserialize)]
struct QueryPage {
    since: Option<i64>,
    #[allow(dead_code)]
    end: Option<i64>,
}

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

// GET /logs/exploit/:id
async fn exploit_one(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> (StatusCode, Json<Value>) {
    let mut conn = state.db.get().unwrap();
    let mut db = Db::new(&mut conn);

    match db.exploits_one(id) {
        Ok(exp) => (StatusCode::OK, json!({ "status": "ok", "data": exp}).into()),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "status": "error", "message": format!("Failed to get exploit: {:?}", e) })
                .into(),
        ),
    }
}

// GET /logs/flags?start=TIMESTAMP
async fn flags(
    State(state): State<Arc<AppState>>,
    query: Query<QueryPage>,
) -> (StatusCode, Json<Value>) {
    let mut conn = state.db.get().unwrap();
    let mut db = Db::new(&mut conn);

    if let Some(since) = query.since {
        let since = match NaiveDateTime::from_timestamp_opt(since, 0) {
            Some(since) => since,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    json!({ "status": "error", "message": "Invalid timestamp" }).into(),
                )
            }
        };

        match db.flags_since(since) {
            Ok(exp) => (StatusCode::OK, json!({ "status": "ok", "data": exp}).into()),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "status": "error", "message": format!("Failed to get flags: {:?}", e) })
                    .into(),
            ),
        }
    } else {
        match db.flags_all() {
            Ok(exp) => (StatusCode::OK, json!({ "status": "ok", "data": exp}).into()),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "status": "error", "message": format!("Failed to get flags: {:?}", e) })
                    .into(),
            ),
        }
    }
}

// GET /logs/executions?start=TIMESTAMP
async fn executions(
    State(state): State<Arc<AppState>>,
    query: Query<QueryPage>,
) -> (StatusCode, Json<Value>) {
    let mut conn = state.db.get().unwrap();
    let mut db = Db::new(&mut conn);

    if let Some(since) = query.since {
        let since = match NaiveDateTime::from_timestamp_opt(since, 0) {
            Some(since) => since,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    json!({ "status": "error", "message": "Invalid timestamp" }).into(),
                )
            }
        };
        match db.executions_since(since) {
            Ok(exp) => (StatusCode::OK, json!({ "status": "ok", "data": exp}).into()),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "status": "error", "message": format!("Failed to get executions: {:?}", e) })
                    .into(),
            ),
        }
    } else {
        match db.executions_all() {
            Ok(exp) => (StatusCode::OK, json!({ "status": "ok", "data": exp}).into()),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "status": "error", "message": format!("Failed to get executions: {:?}", e) })
                    .into(),
            ),
        }
    }
}

// GET /logs/service/:service/exploits
async fn service_exploits(
    State(state): State<Arc<AppState>>,
    Path(service): Path<String>,
) -> (StatusCode, Json<Value>) {
    let mut conn = state.db.get().unwrap();
    let mut db = Db::new(&mut conn);

    match db.service_exploits(&service) {
        Ok(exp) => (StatusCode::OK, json!({ "status": "ok", "data": exp}).into()),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "status": "error", "message": format!("Failed to get exploit: {:?}", e) })
                .into(),
        ),
    }
}

// /logs/
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/exploits", get(exploits_all))
        .route("/exploit/:id", get(exploit_one))
        .route("/flags", get(flags))
        .route("/executions", get(executions))
        .route("/service/:service/exploits", get(service_exploits))
        .with_state(state)
}
