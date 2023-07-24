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

    match db.exploits() {
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

    match db.exploit(id) {
        Ok(exp) => (StatusCode::OK, json!({ "status": "ok", "data": exp}).into()),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "status": "error", "message": format!("Failed to get exploit: {:?}", e) })
                .into(),
        ),
    }
}

// GET /logs/exploit/:id/flags?since=OPTIONAL_TIMESTAMP
async fn exploit_flags(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    query: Query<QueryPage>,
) -> (StatusCode, Json<Value>) {
    let mut conn = state.db.get().unwrap();
    let mut db = Db::new(&mut conn);

    let since = match query.since {
        Some(since) => match NaiveDateTime::from_timestamp_opt(since, 0) {
            Some(since) => since,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    json!({ "status": "error", "message": "Invalid timestamp" }).into(),
                )
            }
        },
        None => NaiveDateTime::from_timestamp_opt(0, 0).unwrap(),
    };

    match db.exploit_flags_since(id, since) {
        Ok(exp) => (StatusCode::OK, json!({ "status": "ok", "data": exp}).into()),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "status": "error", "message": format!("Failed to get flags: {:?}", e) }).into(),
        ),
    }
}

// GET /logs/flags?since=OPTIONAL_TIMESTAMP
async fn flags(
    State(state): State<Arc<AppState>>,
    query: Query<QueryPage>,
) -> (StatusCode, Json<Value>) {
    let mut conn = state.db.get().unwrap();
    let mut db = Db::new(&mut conn);

    let since = match query.since {
        Some(since) => match NaiveDateTime::from_timestamp_opt(since, 0) {
            Some(since) => since,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    json!({ "status": "error", "message": "Invalid timestamp" }).into(),
                )
            }
        },
        None => NaiveDateTime::from_timestamp_opt(0, 0).unwrap(),
    };

    match db.flags_since(since) {
        Ok(exp) => (StatusCode::OK, json!({ "status": "ok", "data": exp}).into()),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "status": "error", "message": format!("Failed to get flags: {:?}", e) }).into(),
        ),
    }
}

// GET /logs/executions?since=OPTIONAL_TIMESTAMP
async fn executions(
    State(state): State<Arc<AppState>>,
    query: Query<QueryPage>,
) -> (StatusCode, Json<Value>) {
    let mut conn = state.db.get().unwrap();
    let mut db = Db::new(&mut conn);

    let since = match query.since {
        Some(since) => match NaiveDateTime::from_timestamp_opt(since, 0) {
            Some(since) => since,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    json!({ "status": "error", "message": "Invalid timestamp" }).into(),
                )
            }
        },
        None => NaiveDateTime::from_timestamp_opt(0, 0).unwrap(),
    };

    match db.executions_since(since) {
        Ok(exp) => (StatusCode::OK, json!({ "status": "ok", "data": exp}).into()),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "status": "error", "message": format!("Failed to get executions: {:?}", e) })
                .into(),
        ),
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

// GET /logs/service/:service/flags?since=OPTIONAL_TIMESTAMP
async fn service_flags(
    State(state): State<Arc<AppState>>,
    Path(service): Path<String>,
    query: Query<QueryPage>,
) -> (StatusCode, Json<Value>) {
    let mut conn = state.db.get().unwrap();
    let mut db = Db::new(&mut conn);

    let since = match query.since {
        Some(since) => match NaiveDateTime::from_timestamp_opt(since, 0) {
            Some(since) => since,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    json!({ "status": "error", "message": "Invalid timestamp" }).into(),
                )
            }
        },
        None => NaiveDateTime::from_timestamp_opt(0, 0).unwrap(),
    };

    match db.service_flags_since(&service, since) {
        Ok(exp) => (StatusCode::OK, json!({ "status": "ok", "data": exp}).into()),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "status": "error", "message": format!("Failed to get flags: {:?}", e) }).into(),
        ),
    }
}

// /logs/
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/exploits", get(exploits_all))
        .route("/exploit/:id", get(exploit_one))
        .route("/exploit/:id/flags", get(exploit_flags))
        .route("/flags", get(flags))
        .route("/executions", get(executions))
        .route("/service/:service/exploits", get(service_exploits))
        .route("/service/:service/flags", get(service_flags))
        .with_state(state)
}
