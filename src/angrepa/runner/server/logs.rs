use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::NaiveDateTime;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use angrepa::db::Db;

use crate::runner::data_types::FlagData;

use super::super::data_types::ExecutionData;
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

    let since = NaiveDateTime::from_timestamp_opt(query.since.unwrap_or(0), 0).unwrap();

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

    let since = NaiveDateTime::from_timestamp_opt(query.since.unwrap_or(0), 0).unwrap();

    let flags =
        match db.flags_since_extended(since) {
            Ok(flags) => flags,
            Err(e) => return (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "status": "error", "message": format!("Failed to get flags: {:?}", e) })
                    .into(),
            ),
        };

    let flags: Vec<FlagData> = flags
        .into_iter()
        .map(|(flag, _exec, target)| FlagData {
            execution_id: flag.execution_id,
            exploit_id: flag.exploit_id,
            id: flag.id,
            status: flag.status,
            submitted: flag.submitted,
            text: flag.text,
            timestamp: flag.timestamp,
            service: target.service,
            target_tick: target.target_tick,
            team: target.team,
        })
        .collect();

    (
        StatusCode::OK,
        json!({ "status": "ok", "data": flags}).into(),
    )
}

#[derive(Deserialize)]
struct FlagIdVector {
    ids: Vec<i32>,
}

// POST /logs/flags
async fn flags_by_id(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<FlagIdVector>,
) -> (StatusCode, Json<Value>) {
    let mut conn = state.db.get().unwrap();
    let mut db = Db::new(&mut conn);

    let flags =
        match db.flags_by_id_extended(payload.ids) {
            Ok(flags) => flags,
            Err(e) => return (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "status": "error", "message": format!("Failed to get flags: {:?}", e) })
                    .into(),
            ),
        };

    let flags: Vec<FlagData> = flags
        .into_iter()
        .map(|(flag, _exec, target)| FlagData {
            execution_id: flag.execution_id,
            exploit_id: flag.exploit_id,
            id: flag.id,
            status: flag.status,
            submitted: flag.submitted,
            text: flag.text,
            timestamp: flag.timestamp,
            service: target.service,
            target_tick: target.target_tick,
            team: target.team,
        })
        .collect();

    (
        StatusCode::OK,
        json!({ "status": "ok", "data": flags}).into(),
    )
}

// GET /logs/executions?since=OPTIONAL_TIMESTAMP
async fn executions(
    State(state): State<Arc<AppState>>,
    query: Query<QueryPage>,
) -> (StatusCode, Json<Value>) {
    let mut conn = state.db.get().unwrap();
    let mut db = Db::new(&mut conn);

    let since = NaiveDateTime::from_timestamp_opt(query.since.unwrap_or(0), 0).unwrap();

    let executions = match db.executions_since_extended(since) {
        Ok(executions) => executions,
        Err(e) => return (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "status": "error", "message": format!("Failed to get executions: {:?}", e) })
                .into(),
        ),
    };

    let executions: Vec<ExecutionData> = executions
        .into_iter()
        .map(|(exec, target, _flag)| ExecutionData::from_models(exec, target))
        .collect();

    (
        StatusCode::OK,
        json!({ "status": "ok", "data": executions}).into(),
    )
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

    let since = NaiveDateTime::from_timestamp_opt(query.since.unwrap_or(0), 0).unwrap();

    match db.service_flags_since(&service, since) {
        Ok(exp) => (StatusCode::OK, json!({ "status": "ok", "data": exp}).into()),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "status": "error", "message": format!("Failed to get flags: {:?}", e) }).into(),
        ),
    }
}

// GET /logs/service/:service/executions?since=OPTIONAL_TIMESTAMP
async fn service_executions(
    State(state): State<Arc<AppState>>,
    Path(service): Path<String>,
    query: Query<QueryPage>,
) -> (StatusCode, Json<Value>) {
    let mut conn = state.db.get().unwrap();
    let mut db = Db::new(&mut conn);

    let since = NaiveDateTime::from_timestamp_opt(query.since.unwrap_or(0), 0).unwrap();

    match db.service_executions_since(&service, since) {
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
        .route("/flags", post(flags_by_id))
        .route("/executions", get(executions))
        .route("/service/:service/exploits", get(service_exploits))
        .route("/service/:service/flags", get(service_flags))
        .route("/service/:service/executions", get(service_executions))
        .with_state(state)
}
