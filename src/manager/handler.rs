//! The handler is responsible for
//! - Extracting flags from raw data
//! - Regularly sending flags to the submitter
//!
//! It therefore consists of a few parts, running in parallel:
//! - A getter *routine*, which takes raw data (from listeners), and gives flags
//! - A submitter function, which sends the passwed flags to the submitter

use std::collections::HashMap;

use regex::Regex;
use tokio::{select, spawn};
use tracing::info;

use crate::{
    submitter::{FlagStatus, Submitter},
    Flag, Manager,
};

/// Extracts flags from raw input
async fn getter(
    raw_flag_rx: flume::Receiver<String>,
    parsed_flag_tx: flume::Sender<Flag>,
    flag_regex: Regex,
) {
    while let Ok(raw) = raw_flag_rx.recv_async().await {
        for flag in flag_regex.captures_iter(&raw) {
            let flag = flag[0].to_string();
            info!("Recieved flag {}", flag);
            let flag = Flag {
                flag,
                ..Default::default()
            };
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
    manager: Manager,
    raw_flag_rx: flume::Receiver<String>,
    submitter: impl Submitter + Send + Sync + Clone + 'static,
    flag_regex: Regex,
) {
    // set up channelsStrinStrin
    let (parsed_tx, parsed_rx) = flume::unbounded::<Flag>();
    let (result_tx, result_rx) = flume::unbounded::<(String, FlagStatus)>();

    // spawn the getter
    spawn(getter(raw_flag_rx, parsed_tx, flag_regex));

    let mut status: HashMap<String, FlagStatus> = HashMap::new();
    let mut flag_queue = Vec::new();

    // submit every 5s
    let mut send_signal = tokio::time::interval(std::time::Duration::from_secs(5));
    send_signal.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

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
                // add to db
                let new = manager.register_flag(f.clone());

                if new {
                    flag_queue.push(f.flag);
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
