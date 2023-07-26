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
use exploit::{docker::InitalizedExploit, Exploit};

mod server;

pub struct Runner {}

impl Runner {
    async fn tick(flag_regex: Regex, db_url: &String, oldest_possible_flags: i64) {
        let mut conn = db_connect(db_url).unwrap();
        let mut db = Db::new(&mut conn);

        let docker = Docker::connect_with_local_defaults().unwrap();

        let oldest =
            chrono::Utc::now().naive_utc() - chrono::Duration::seconds(oldest_possible_flags);

        let targets = match db.get_exploitable_target(oldest) {
            Ok(targets) => targets,
            Err(err) => {
                warn!("Failed to get exploitable targets: {:?}", err);
                return;
            }
        };

        for (targets, exploit) in targets {
            let docker = docker.clone();

            let mut instance =
                InitalizedExploit::from_model(docker, exploit.clone(), Db::new(&mut conn))
                    .await
                    .unwrap();

            for target in targets {
                let flag_regex = flag_regex.clone();
                let db_url = db_url.to_owned();

                let run = instance.run(target.team, target.flag_id).await;

                let log_future = match run {
                    Ok(run) => run,
                    Err(err) => {
                        warn!("Failed to run exploit: {:?}", err);
                        continue;
                    }
                };

                tokio::spawn(async move {
                    let started_at = chrono::Utc::now().naive_utc();

                    let log = log_future.await.unwrap();

                    let finished_at = chrono::Utc::now().naive_utc();

                    let mut conn = db_connect(&db_url).unwrap();
                    let mut db = Db::new(&mut conn);
                    let execution = db
                        .add_execution(&ExecutionInserter {
                            exploit_id: exploit.id,
                            output: log.output.clone(),
                            started_at,
                            finished_at,
                            target_id: target.id,
                        })
                        .unwrap();

                    db.target_exploited(target.id).unwrap();

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
            let db_url = config.database.url();
            let oldest_possible_flags = config.common.oldest_possible_flags;
            spawn(async move { Runner::tick(flag_regex, &db_url, oldest_possible_flags).await });
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
    let db_url = config.database.url();
    let server_handle = spawn(async move { server::run(server_addr, &db_url).await });

    let runner_handle = spawn(async move { Runner::run(&config).await });

    join_all(vec![runner_handle, server_handle]).await;

    Ok(())
}
