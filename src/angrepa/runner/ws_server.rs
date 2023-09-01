use angrepa::{config, db::Db, db_connect};
use bus::Bus;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, json};
use sqlx::postgres::{PgListener, PgPoolOptions};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use tracing::{error, info};
use tungstenite::{accept, Message};

pub async fn wslistener(addr: std::net::SocketAddr, bus: Arc<Mutex<Bus<String>>>) {
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
#[derive(Serialize, Deserialize)]
struct DbTrigger {
    table: String,
    id: i32,
}

pub async fn run(config: config::Root, addr: std::net::SocketAddr) {
    let bus = Arc::new(Mutex::new(Bus::<String>::new(100)));
    let bus_copy = bus.clone();

    tokio::spawn(async move { wslistener(addr, bus_copy).await });

    tokio::spawn(async move {
        let sqlxpool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&config.database.url())
            .await
            .unwrap();

        let mut listener = PgListener::connect_with(&sqlxpool).await.unwrap();
        listener.listen("db_notifications").await.unwrap();

        info!("Spawned DB listener");

        loop {
            while let Ok(notification) = listener.recv().await {
                match from_str::<DbTrigger>(notification.payload()) {
                    Ok(data) => {
                        let mut conn = db_connect(&config.database.url()).unwrap();
                        let mut db = Db::new(&mut conn);

                        match data.table.as_str() {
                            "exploit" => {
                                let exp = db.exploit(data.id).unwrap();
                                let mut bus = bus.lock().unwrap();
                                bus.broadcast(
                                    json!({"table": data.table, "data": exp}).to_string(),
                                );
                            }
                            "flag" => {
                                let flag = db.flags_by_id_extended(vec![data.id]).unwrap();
                                let mut bus = bus.lock().unwrap();
                                bus.broadcast(
                                    json!({"table": data.table, "data": flag}).to_string(),
                                );
                            }
                            "execution" => {
                                let exec = db.executions_by_id_extended(vec![data.id]).unwrap();
                                let mut bus = bus.lock().unwrap();
                                bus.broadcast(
                                    json!({"table": data.table, "data": exec}).to_string(),
                                );
                            }
                            _ => (),
                        }
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
