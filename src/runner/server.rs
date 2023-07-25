use axum::Router;
use diesel::r2d2::ConnectionManager;
use diesel::r2d2::Pool;
use diesel::PgConnection;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

use angrepa::get_connection_pool;

mod exploit;
mod logs;
mod ping;
mod templates;

pub struct AppState {
    db: Pool<ConnectionManager<PgConnection>>,
}

pub async fn run(addr: std::net::SocketAddr, db_url: &String) {
    let app_state = Arc::new(AppState {
        db: get_connection_pool(db_url).unwrap(),
    });

    let app = Router::new()
        .nest("/templates", templates::router())
        .nest("/exploit", exploit::router(Arc::clone(&app_state)))
        .nest("/logs", logs::router(app_state))
        .nest("/ping", ping::router())
        .layer(CorsLayer::new().allow_methods(Any).allow_origin(Any));

    tracing::info!("Webserver started on {addr}");
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
