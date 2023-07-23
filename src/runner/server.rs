use axum::Router;
use std::net::SocketAddr;

mod exploit;
mod templates;

pub async fn run() {
    let app = Router::new()
        .nest("/templates", templates::router())
        .nest("/exploit", exploit::router());

    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));
    tracing::info!("Webserver started on {addr}");
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
