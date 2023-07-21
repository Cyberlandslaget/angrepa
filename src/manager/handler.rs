//! The handler is responsible for
//! - Extracting flags from raw data
//! - Regularly sending flags to the submitter
//!
//! It therefore consists of a few parts, running in parallel:
//! - A getter *routine*, which takes raw data (from listeners), and gives flags
//! - A submitter function, which sends the passwed flags to the submitter

use std::collections::{HashMap, HashSet};

use angrapa::{db_connect, models::FlagModel};
use diesel::RunQueryDsl;
use regex::Regex;
use tokio::{select, spawn};
use tracing::info;

use crate::submitter::{FlagStatus, Submitter};

/// Extracts flags from raw input
async fn getter(
    raw_flag_rx: flume::Receiver<String>,
    parsed_flag_tx: flume::Sender<String>,
    flag_regex: Regex,
) {
    while let Ok(raw) = raw_flag_rx.recv_async().await {
        for flag in flag_regex.captures_iter(&raw) {
            let flag = flag[0].to_string();
            info!("Recieved flag {}", flag);
            parsed_flag_tx.send_async(flag).await.unwrap();
        }
    }
}

/// Submits flags
async fn submit(
    submitter: impl Submitter + Send + Sync + Clone + 'static,
    flags: Vec<String>,
    result_tx: flume::Sender<(String, FlagStatus)>,
) {
    info!("Submitting {:?}", flags);
    let results = submitter.submit(flags).await.unwrap();
    for res in results {
        result_tx.send_async(res).await.unwrap();
    }
}

pub async fn run(
    raw_flag_rx: flume::Receiver<String>,
    submitter: impl Submitter + Send + Sync + Clone + 'static,
    flag_regex: Regex,
) {
    // set up channels
    let (parsed_tx, parsed_rx) = flume::unbounded::<String>();
    let (result_tx, result_rx) = flume::unbounded::<(String, FlagStatus)>();

    // spawn the getter
    spawn(getter(raw_flag_rx, parsed_tx, flag_regex));

    // TODO TODO TODO get this from the db !!!
    let mut seen: HashSet<String> = HashSet::new();
    let mut status: HashMap<String, FlagStatus> = HashMap::new();
    let mut flag_queue = Vec::new();

    // submit every 5s
    let mut send_signal = tokio::time::interval(std::time::Duration::from_secs(5));
    send_signal.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let db = &mut db_connect().unwrap();

    loop {
        select!(
            _ = send_signal.tick() => {
                let to_submit = flag_queue.clone();
                flag_queue.clear();

                let result_tx = result_tx.clone();

                spawn(submit(submitter.clone(), to_submit, result_tx));
            },
            f = parsed_rx.recv_async() => {
                let f = f.unwrap();
                if seen.insert(f.clone()) {
                    flag_queue.push(f.clone());

                    // insert into db

                    let model = FlagModel { flag: f.clone(), tick: None, stamp: None, exploit_id: None, target_ip: None, flagstore: None};

                    let fm: FlagModel = diesel::insert_into(angrapa::schema::flags::table)
                        .values(model)
                        .returning(angrapa::schema::flags::all_columns)
                        .get_result(db)
                        .unwrap();

                    info!("Inserted flag {:?} into db", fm);
                }
            },
            res = result_rx.recv_async() => {
                let (flag, flag_status) = res.unwrap();
                status.insert(flag.clone(), flag_status);
                info!("Got status  {}: {:?}", flag, flag_status);
            }
        )
    }
}
