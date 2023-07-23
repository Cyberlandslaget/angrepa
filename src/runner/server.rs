use axum::Router;

mod exploit;
mod templates;

pub async fn run(addr: std::net::SocketAddr) {
    let app = Router::new()
        .nest("/templates", templates::router())
        .nest("/exploit", exploit::router());

    tracing::info!("Webserver started on {addr}");
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
