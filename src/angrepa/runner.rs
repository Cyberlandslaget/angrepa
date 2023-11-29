use std::collections::HashSet;

use bollard::Docker;
use chrono::NaiveDateTime;
use regex::Regex;

use color_eyre::Report;
use futures::{future::join_all, StreamExt};
use tokio::{spawn, time::timeout};
use tracing::{info, warn};

use angrepa::{
    config::{self},
    db_connect,
    inserter::{FlagInserter, ExecutionInserter},
};

mod exploit;
use exploit::{docker::InitalizedExploit, Exploit};

use self::exploit::RunLog;

mod server;
mod ws_server;

pub struct Runner {}

impl Runner {
    async fn tick(config: config::Root, flag_regex: Regex, earliest_valid_time: NaiveDateTime) {
        let db = db_connect(&config.database.url()).await.unwrap();

        let docker = Docker::connect_with_local_defaults().unwrap();

        let targets = match db
            .get_exploitable_targets_updating(earliest_valid_time)
            .await
        {
            Ok(targets) => targets,
            Err(err) => {
                warn!("Failed to get exploitable targets: {:?}", err);
                return;
            }
        };

        for (targets, exploit) in targets {
            let docker = docker.clone();

            let mut instance = InitalizedExploit::from_model(docker, exploit.clone(), db.clone())
                .await
                .unwrap();

            let blacklist: HashSet<_> = exploit.blacklist.iter().collect();

            for target in targets {
                if blacklist.contains(&target.team) {
                    continue;
                }

                let flag_regex = flag_regex.clone();

                let run = instance
                    .run(&config.common, target.team, target.flag_id)
                    .await;

                let (exec_future, rx) = match run {
                    Ok(run) => run,
                    Err(err) => {
                        warn!("Failed to run exploit: {:?}", err);
                        continue;
                    }
                };

                let db = db.clone();

                tokio::spawn(async move {
                    let started_at = chrono::Utc::now().naive_utc();

                    // a long ass time, should never happen that it doesnt quit before this due to other timeout mesaures, but we should be notified if it doesnt
                    let exec = timeout(tokio::time::Duration::from_secs(600), exec_future).await;
                    let exec = exec.map(|inner| inner.unwrap());

                    let mut logs: String = rx.stream().collect().await;

                    let exec = match exec {
                        Ok(exec) => exec,
                        Err(_) => {
                            warn!("Execution didn't stop after 10 minutes. Quite bad!");
                            logs += "angrepa: listener killed due to timeout. this is bad!";
                            RunLog { exit_code: 0 }
                        }
                    };

                    let finished_at = chrono::Utc::now().naive_utc();

                    let execution = db
                        .add_execution(&ExecutionInserter {
                            exploit_id: exploit.id,
                            output: logs.clone().replace('\x00', ""), // fix this ugly shit
                            exit_code: exec.exit_code as i32,
                            started_at,
                            finished_at,
                            target_id: target.id,
                        })
                        .await
                        .unwrap();

                    // only unique flags
                    let flags: HashSet<String> = flag_regex
                        .captures_iter(&logs)
                        .map(|cap| cap[0].to_string())
                        .collect();

                    for flag in flags {
                        db.add_flag(&FlagInserter {
                            text: flag,
                            status: "".to_string(),
                            submitted: false,
                            timestamp: chrono::Utc::now().naive_utc(),
                            execution_id: execution.id,
                            exploit_id: exploit.id,
                        })
                        .await
                        .unwrap();
                    }
                });
            }
        }
    }

    async fn run(config: &config::Root) {
        let mut tick_interval = config
            .common
            // make sure the tick has started
            .get_tick_interval(tokio::time::Duration::from_secs(1))
            .await
            .unwrap();

        let flag_regex = Regex::new(&config.common.format).unwrap();

        loop {
            tick_interval.tick().await;

            let flag_regex = flag_regex.clone();

            // mid inbetween so that if we start ex. 0.01s earlier than last tick, we dont take too many
            // -0.5 because of the afformentioned in-betweening
            let flag_validity_period =
                (config.common.flag_validity as f32 - 0.5) * (config.common.tick as f32);
            let flag_validity_period = std::time::Duration::from_secs_f32(flag_validity_period);

            let earliest_valid_time = chrono::Utc::now().naive_utc()
                - chrono::Duration::from_std(flag_validity_period).unwrap();

            let config = config.clone();

            spawn(async move { Runner::tick(config, flag_regex, earliest_valid_time).await });
        }
    }
}

pub async fn main(config: config::Root) -> Result<(), Report> {
    let common = &config.common;

    // time until start
    common.sleep_until_start().await;
    assert!(chrono::Utc::now() >= common.start);

    let time_since_start = chrono::Utc::now() - common.start;
    info!("CTF started {:?} ago", time_since_start);

    let server_addr = config.runner.http_server.parse()?;
    let config2 = config.clone();
    let server_handle = spawn(async move { server::run(server_addr, config2).await });

    let ws_addr = config.runner.ws_server.parse()?;
    let config2 = config.clone();
    let ws_handle = spawn(async move { ws_server::run(config2, ws_addr).await });

    let runner_handle = spawn(async move { Runner::run(&config).await });

    join_all(vec![runner_handle, server_handle, ws_handle]).await;

    Ok(())
}
