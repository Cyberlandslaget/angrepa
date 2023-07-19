use color_eyre::Report;
use futures::future::join_all;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use tokio::select;
use tokio::task::spawn;

mod submitter;
use submitter::FlagStatus;

mod listener;
use listener::{Tcp, Web};

use crate::submitter::{Submitter, Submitters};

struct SubmitterManager<S> {
    submitter: S,
    flag_regex: Regex,
}

impl<S> SubmitterManager<S>
where
    S: Submitter + Send + Sync + Clone + 'static,
{
    /// Submits flags
    async fn submit(
        submitter: S,
        flags: Vec<String>,
        result_tx: flume::Sender<(String, FlagStatus)>,
    ) {
        println!("Submitting {:?}", flags);
        let results = submitter.submit(flags).await.unwrap();
        for res in results {
            result_tx.send_async(res).await.unwrap();
        }
    }

    /// Extracts out flags
    async fn getter(
        raw_flag_rx: flume::Receiver<String>,
        parsed_tx: flume::Sender<String>,
        flag_regex: Regex,
    ) {
        while let Ok(raw) = raw_flag_rx.recv_async().await {
            for flag in flag_regex.captures_iter(&raw) {
                let flag = flag[0].to_string();
                println!("Recieved flag {}", flag);
                parsed_tx.send_async(flag).await.unwrap();
            }
        }
    }

    async fn run(&self, raw_flag_rx: flume::Receiver<String>) {
        let (parsed_tx, parsed_rx) = flume::unbounded::<String>();
        let (result_tx, result_rx) = flume::unbounded::<(String, FlagStatus)>();

        spawn(SubmitterManager::<S>::getter(
            raw_flag_rx,
            parsed_tx,
            self.flag_regex.clone(),
        ));

        let mut seen: HashSet<String> = HashSet::new();
        let mut status: HashMap<String, FlagStatus> = HashMap::new();
        let mut flag_queue = Vec::new();

        let mut send_signal = tokio::time::interval(std::time::Duration::from_secs(5));
        send_signal.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            select!(
                _ = send_signal.tick() => {
                    let mut to_submit = Vec::new();
                    std::mem::swap(&mut flag_queue, &mut to_submit);

                    let result_tx = result_tx.clone();

                    spawn(SubmitterManager::<S>::submit(self.submitter.clone(), to_submit, result_tx));
                },
                f = parsed_rx.recv_async() => {
                    let f = f.unwrap();
                    if seen.insert(f.clone()) {
                        flag_queue.push(f);
                    }
                },
                res = result_rx.recv_async() => {
                    let (flag, flag_status) = res.unwrap();
                    status.insert(flag.clone(), flag_status);
                    println!("Got status  {}: {:?}", flag, flag_status);
                }
            )
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Report> {
    color_eyre::install()?;

    // get toml
    let args = argh::from_env::<angrapa::config::Args>();
    let toml = std::fs::read_to_string(args.toml)?;
    let config = toml::from_str::<angrapa::config::Root>(&toml)?;

    let flag_regex = Regex::new(&config.common.format)?;

    println!("manager");

    let sub = Submitters::from_conf(&config.manager)?;

    // set up channels
    let (flag_tx, flag_rx) = flume::unbounded::<String>();

    // run tcp listener on another thread
    let tcp_handle = {
        let flag_tx = flag_tx.clone();

        let tcp = Tcp::new(config.manager.tcp_listener.parse()?);
        tokio::spawn(async move {
            tcp.run(flag_tx).await.unwrap();
        })
    };

    // run web listener on another thread
    let web_handle = {
        let flag_tx = flag_tx.clone();
        let web = Web::new(config.manager.http_listener.parse()?);

        tokio::spawn(async move {
            web.run(flag_tx).await.unwrap();
        })
    };

    // run submitter on another thread
    let sub_handle = tokio::spawn(async move {
        match sub {
            Submitters::Dummy(submitter) => {
                SubmitterManager {
                    submitter,
                    flag_regex,
                }
                .run(flag_rx)
                .await
            }
            Submitters::ECSC(submitter) => {
                SubmitterManager {
                    submitter,
                    flag_regex,
                }
                .run(flag_rx)
                .await
            }
        }
    });

    // join all
    join_all(vec![tcp_handle, web_handle, sub_handle]).await;

    Ok(())
}
