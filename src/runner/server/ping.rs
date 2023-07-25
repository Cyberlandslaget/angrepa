use axum::{routing::get, Router};
use reqwest::StatusCode;

pub fn router() -> Router {
    Router::new().route("/", get(ping))
}

pub async fn ping() -> (StatusCode, &'static str) {
    (StatusCode::OK, "pong")
}
