use angrepa::config;
use axum::{http::StatusCode, routing::get, Router};
use diesel::r2d2::ConnectionManager;
use diesel::r2d2::Pool;
use diesel::PgConnection;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

use angrepa::get_connection_pool;

mod exploit;
mod info;
mod logs;
mod templates;

pub struct AppState {
    db: Pool<ConnectionManager<PgConnection>>,
    config: config::Root,
}

pub async fn run(addr: std::net::SocketAddr, config: config::Root, db_url: &String) {
    let app_state = Arc::new(AppState {
        db: get_connection_pool(db_url).unwrap(),
        config,
    });

    let app = Router::new()
        .route("/ping", get(|| async { (StatusCode::OK, "pong") }))
        .nest("/templates", templates::router())
        .nest("/exploit", exploit::router(Arc::clone(&app_state)))
        .nest("/logs", logs::router(Arc::clone(&app_state)))
        .nest("/info", info::router(app_state))
        .layer(CorsLayer::new().allow_methods(Any).allow_origin(Any));

    tracing::info!("Webserver started on {addr}");
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
