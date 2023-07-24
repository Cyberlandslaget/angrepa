use axum::Router;
use tower_http::cors::{Any, CorsLayer};

use super::CONFIG;

mod exploit;
mod templates;

pub async fn run() {
    let app = Router::new()
        .nest("/templates", templates::router())
        .nest("/exploit", exploit::router())
        .layer(CorsLayer::new().allow_methods(Any).allow_origin(Any));

    let addr = CONFIG.runner.http_server.parse().unwrap();

    tracing::info!("Webserver started on {addr}");
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
