use angrepa::config;
use bus::Bus;
use serde_json::from_str;
use sqlx::postgres::{PgListener, PgPoolOptions};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use tracing::{error, info};
use tungstenite::{accept, Message};

pub async fn wslistener(addr: std::net::SocketAddr, bus: Arc<Mutex<Bus<serde_json::Value>>>) {
    let server = TcpListener::bind(addr).unwrap();
    info!("WS server started on {:?}", addr);

    for stream in server.incoming() {
        let mut rx = bus.lock().unwrap().add_rx();

        std::thread::spawn(move || {
            let mut websocket = accept(stream.unwrap()).unwrap();
            loop {
                let msg = rx.recv().unwrap().to_string();
                match websocket.send(Message::Text(msg)) {
                    Ok(_) => (),
                    Err(_) => {
                        // error, so quit
                        break;
                    }
                }
            }
        });
    }
}

pub async fn run(config: config::Root, addr: std::net::SocketAddr) {
    let bus = Arc::new(Mutex::new(Bus::<serde_json::Value>::new(100)));
    let bus_copy = bus.clone();

    tokio::spawn(async move { wslistener(addr, bus_copy).await });

    tokio::spawn(async move {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&config.database.url())
            .await
            .unwrap();

        let mut listener = PgListener::connect_with(&pool).await.unwrap();
        listener.listen("db_notifications").await.unwrap();

        info!("Spawned DB listener");

        loop {
            while let Ok(notification) = listener.recv().await {
                match from_str::<serde_json::Value>(&notification.payload()) {
                    Ok(data) => {
                        let mut bus = bus.lock().unwrap();
                        bus.broadcast(data);
                    }
                    Err(e) => error!(
                        "Failed to parse notification: {}.The payload was: '{:?}'",
                        e,
                        notification.payload()
                    ),
                }
            }
        }
    });
}
