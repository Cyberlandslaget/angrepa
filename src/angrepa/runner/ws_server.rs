use angrepa::{config, db::Db, db_connect};
use chrono::{NaiveDateTime, Utc};
use std::{collections::HashMap, net::TcpListener};
use tracing::{info, warn};
use tungstenite::{accept, Message};

// TODO list
// [x] flag
// [ ] execution
// [ ] exploits (uploaded / endret)
// [ ] tick

pub async fn listener(addr: std::net::SocketAddr, chan: flume::Sender<flume::Sender<String>>) {
    let server = TcpListener::bind(addr).unwrap();
    info!("WS server started on {:?}", addr);

    for stream in server.incoming() {
        let (tx, rx) = flume::unbounded();
        chan.send_async(tx).await.unwrap();
        std::thread::spawn(move || {
            let mut websocket = accept(stream.unwrap()).unwrap();
            loop {
                let msg = rx.recv().unwrap();
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

#[derive(serde::Serialize, PartialEq, Clone)]
struct FlagData {
    execution_id: i32,
    exploit_id: i32,
    id: i32,
    status: String,
    submitted: bool,
    text: String,
    timestamp: NaiveDateTime,
    service: String,
    target_tick: i32,
    team: String,
}

struct FlagGetter {
    last_response: HashMap<i32, FlagData>,
}

impl FlagGetter {
    pub fn new() -> Self {
        Self {
            last_response: HashMap::new(),
        }
    }

    pub fn get(&mut self, db: &mut Db, since: NaiveDateTime) -> Vec<FlagData> {
        let flags = match db.flags_since_extended(since) {
            Ok(flags) => flags,
            Err(e) => {
                warn!("Failed to get flags {:?}", e);
                return vec![];
            }
        };

        let flags: Vec<_> = flags
            .into_iter()
            .map(|(flag, _exec, target)| FlagData {
                execution_id: flag.execution_id,
                exploit_id: flag.exploit_id,
                id: flag.id,
                status: flag.status,
                submitted: flag.submitted,
                text: flag.text,
                timestamp: flag.timestamp,
                service: target.service,
                target_tick: target.target_tick,
                team: target.team,
            })
            .filter(|flag| self.last_response.get(&flag.id) != Some(flag))
            .collect();

        for flag in &flags {
            self.last_response.insert(flag.id, flag.to_owned());
        }

        flags
    }
}

pub async fn run(config: config::Root, addr: std::net::SocketAddr) {
    let mut conn = db_connect(&config.database.url()).unwrap();
    let mut db = Db::new(&mut conn);

    let mut listeners: Vec<flume::Sender<String>> = Vec::new();

    let (tx, rx) = flume::unbounded();
    tokio::spawn(async move { listener(addr, tx).await });

    let mut flag_getter = FlagGetter::new();

    loop {
        // without this the other func wont even run
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let since = (Utc::now() - chrono::Duration::seconds(60)).naive_utc();

        let flags = flag_getter.get(&mut db, since);
        let flags = serde_json::to_string(&flags).unwrap();

        // add any new listeners
        while let Ok(exec) = rx.try_recv() {
            listeners.push(exec);
        }

        // send it
        let mut new_listeners = vec![];

        for l in listeners {
            if l.send_async(flags.clone()).await.is_ok() {
                new_listeners.push(l);
            }
        }

        listeners = new_listeners;
    }
}
