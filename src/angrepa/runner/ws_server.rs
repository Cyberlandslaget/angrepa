use angrepa::data_types::{ExecutionData, ExploitData, FlagData};
use angrepa::{config, db::Db, db_connect};
use chrono::{NaiveDateTime, Utc};
use serde_json::json;
use std::{collections::HashMap, net::TcpListener};
use tracing::{info, warn};
use tungstenite::{accept, Message};

// TODO list
// [x] flag
// [x] execution
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
            .map(|(flag, _exec, target)| FlagData::from_models(flag, target))
            .filter(|flag| self.last_response.get(&flag.id) != Some(flag))
            .collect();

        for flag in &flags {
            self.last_response.insert(flag.id, flag.to_owned());
        }

        flags
    }
}

struct ExecutionGetter {
    last_response: HashMap<i32, ExecutionData>,
}

impl ExecutionGetter {
    pub fn new() -> Self {
        Self {
            last_response: HashMap::new(),
        }
    }

    pub fn get(&mut self, db: &mut Db, since: NaiveDateTime) -> Vec<ExecutionData> {
        let executions = match db.executions_since_extended(since) {
            Ok(exs) => exs,
            Err(e) => {
                warn!("Failed to get executions {:?}", e);
                return vec![];
            }
        };

        let executions: Vec<ExecutionData> = executions
            .into_iter()
            .map(|(exec, target, _flag)| ExecutionData::from_models(exec, target))
            .filter(|exec| self.last_response.get(&exec.id) != Some(exec))
            .collect();

        for exec in &executions {
            self.last_response.insert(exec.id, exec.to_owned());
        }

        executions
    }
}

struct ExploitGetter {
    last_response: HashMap<i32, ExploitData>,
}

impl ExploitGetter {
    pub fn new() -> Self {
        Self {
            last_response: HashMap::new(),
        }
    }

    pub fn get(&mut self, db: &mut Db) -> Vec<ExploitData> {
        let expls = match db.exploits() {
            Ok(expls) => expls,
            Err(e) => {
                warn!("Failed to get flags {:?}", e);
                return vec![];
            }
        };

        let flags: Vec<_> = expls
            .into_iter()
            .map(|exploit| ExploitData::from_model(exploit))
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
    let mut exec_getter = ExecutionGetter::new();
    let mut expl_getter = ExploitGetter::new();

    loop {
        // without this the other func wont even run
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let since = (Utc::now() - chrono::Duration::seconds(60)).naive_utc();

        let flags = flag_getter.get(&mut db, since);
        let execs = exec_getter.get(&mut db, since);
        let expls = expl_getter.get(&mut db);

        let txt = serde_json::to_string(&json!({
            "flags": flags,
            "executions": execs,
            "exploits": expls,
        }))
        .unwrap();

        // add any new listeners
        while let Ok(exec) = rx.try_recv() {
            listeners.push(exec);
        }

        // send it
        let mut new_listeners = vec![];

        for l in listeners {
            if l.send_async(txt.clone()).await.is_ok() {
                new_listeners.push(l);
            }
        }

        listeners = new_listeners;
    }
}
