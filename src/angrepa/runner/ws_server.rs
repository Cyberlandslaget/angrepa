use angrepa::config;
use serde_json::from_str;
use sqlx::postgres::{PgListener, PgPoolOptions};
use tracing::{error, info, warn};

pub async fn run(config: config::Root, _addr: std::net::SocketAddr) {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database.url())
        .await
        .unwrap();

    tokio::spawn(async move {
        let mut listener = PgListener::connect_with(&pool).await.unwrap();
        listener.listen("db_notifications").await.unwrap();

        info!("Spawned DB listener");

        loop {
            while let Ok(notification) = listener.recv().await {
                match from_str::<serde_json::Value>(&notification.payload()) {
                    Ok(data) => warn!("{:?}", data), // TODO: Push WS notification to clients
                    Err(e) => error!("Failed to parse notification: {}", e),
                }
            }
        }
    });
}
