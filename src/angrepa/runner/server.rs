use angrepa::{config, db_connect};
use angrepa::db::Db;
use axum::{http::StatusCode, routing::get, Router};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

mod exploit;
mod info;
mod logs;
mod templates;

pub struct AppState {
    db: Db,
    config: config::Root,
}

pub async fn run(addr: std::net::SocketAddr, config: config::Root) {
    let db = db_connect(&config.database.url()).await.unwrap();

    let app_state = Arc::new(AppState { db, config });

    let app = Router::new()
        .route("/ping", get(|| async { (StatusCode::OK, "pong") }))
        .nest("/templates", templates::router())
        .nest("/exploit", exploit::router(Arc::clone(&app_state)))
        .nest("/logs", logs::router(Arc::clone(&app_state)))
        .nest("/info", info::router(app_state))
        .layer(
            CorsLayer::new()
                .allow_methods(Any)
                .allow_origin(Any)
                .allow_private_network(true)
                .allow_headers(Any),
        );

    info!("Webserver started on {addr}");
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
