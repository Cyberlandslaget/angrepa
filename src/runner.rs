use bollard::Docker;
use regex::Regex;

use color_eyre::Report;
use futures::future::join_all;
use tokio::spawn;
use tracing::{info, warn};

use angrepa::{
    config::{self},
    db::Db,
    db_connect,
    models::{ExecutionInserter, FlagInserter},
};

mod exploit;
use exploit::exploit2::{docker::InitalizedExploit, Exploit};

mod server;

use crate::manager::Manager;

pub struct Runner {}

impl Runner {
    async fn tick(manager: Manager, flag_regex: Regex, db_url: &String) {
        let mut conn = db_connect(db_url).unwrap();
        let mut db = Db::new(&mut conn);

        let exploits = db.get_exploits().unwrap();

        let docker = Docker::connect_with_local_defaults().unwrap();

        for exploit in exploits {
            if !exploit.enabled {
                continue;
            }

            let targets = manager.get_service_targets(&exploit.service);

            let mut going_to_exploit = Vec::new();
            if let Some(targets) = targets {
                // the service was found and we got all the targets
                for (host, ticks) in targets.0.iter() {
                    // get the latest tick
                    let (_tick_value, flag_id) = match ticks.get_latest() {
                        Some((tick_value, flag_id)) => (tick_value, flag_id),
                        None => {
                            warn!("No tick found for host {}", host);
                            continue;
                        }
                    };

                    // dump as string
                    let flag_id = flag_id.to_string();

                    // add this host and the flag_id to the list of targets that will be attacked
                    going_to_exploit.push((host.to_owned(), flag_id));
                }
            } else {
                // service not known
                // instead, run against all ips, without any flagids

                let ips = manager.all_ips();
                for ip in ips {
                    going_to_exploit.push((ip, "".to_string()));
                }
            }

            let docker = docker.clone();
            let instance = InitalizedExploit::from_model(docker, exploit.clone())
                .await
                .unwrap();

            for (target_host, target_flagid) in going_to_exploit {
                let instance = instance.clone();
                let flag_regex = flag_regex.clone();
                let db_url = db_url.clone();

                tokio::spawn(async move {
                    let started_at = chrono::Utc::now().naive_utc();

                    let log = instance
                        .run_till_completion(target_host.to_string(), target_flagid.to_string())
                        .await
                        .unwrap();

                    let finished_at = chrono::Utc::now().naive_utc();

                    let mut conn = db_connect(&db_url).unwrap();
                    let mut db = Db::new(&mut conn);
                    let execution = db
                        .add_execution(&ExecutionInserter {
                            exploit_id: exploit.id,
                            output: log.output.clone(),
                            started_at,
                            finished_at,
                        })
                        .unwrap();

                    // find flags in the string
                    let flags = flag_regex
                        .captures_iter(&log.output)
                        .map(|cap| cap[0].to_string());

                    for flag in flags {
                        db.add_flag(&FlagInserter {
                            text: flag,
                            status: "".to_string(),
                            submitted: false,
                            timestamp: chrono::Utc::now().naive_utc(),
                            execution_id: execution.id,
                            exploit_id: exploit.id,
                        })
                        .unwrap();
                    }
                });
            }
        }
    }

    async fn run(manager: Manager, config: &config::Root) {
        let mut tick_interval = config
            .common
            // make sure the tick has started
            .get_tick_interval(tokio::time::Duration::from_secs(1))
            .await
            .unwrap();

        let flag_regex = Regex::new(&config.common.format).unwrap();

        loop {
            let manager = manager.clone();
            tick_interval.tick().await;

            let manager = manager.clone();
            let flag_regex = flag_regex.clone();
            let db_url = config.database.url();
            spawn(async move { Runner::tick(manager, flag_regex, &db_url).await });
        }
    }
}

pub async fn main(config: config::Root, manager: Manager) -> Result<(), Report> {
    let common = &config.common;

    // time until start
    common.sleep_until_start().await;
    assert!(chrono::Utc::now() >= common.start);
    info!("Manager woke up!");

    let time_since_start = chrono::Utc::now() - common.start;
    info!("CTF started {:?} ago", time_since_start);

    let server_addr = config.runner.http_server.parse()?;
    let db_url = config.database.url();
    let server_handle = spawn(async move { server::run(server_addr, &db_url).await });

    let runner_handle = spawn(async move { Runner::run(manager, &config).await });

    join_all(vec![runner_handle, server_handle]).await;

    Ok(())
}

#[allow(dead_code)]
fn tarify(path: &str) -> Result<Vec<u8>, Report> {
    use tar::Builder;

    let mut tar = Builder::new(Vec::new());

    tar.append_dir_all(".", path)?;
    tar.finish()?;

    let tar = tar.into_inner()?;
    Ok(tar)
}
