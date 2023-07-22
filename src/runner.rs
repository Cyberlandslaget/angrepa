use bollard::Docker;
use regex::Regex;

use color_eyre::Report;
use futures::future::join_all;
use tokio::spawn;
use tracing::{info, warn};

use angrapa::{
    config::{self, Common},
    db::Db,
    db_connect,
    models::ExecutionInserter,
};

mod exploit;
use exploit::exploit2::{
    docker::{DockerExploit, DockerExploitPool},
    Exploit, ExploitInstance,
};

//mod server;
//use server::Server;

use crate::manager::Manager;

#[derive(Debug, Clone)]
pub enum Exploits {
    DockerPool(DockerExploitPool),
    Docker(DockerExploit),
}

impl Exploits {
    pub fn as_str(&self) -> String {
        match self {
            Exploits::DockerPool(_) => "docker_pool".to_string(),
            Exploits::Docker(_) => "docker".to_string(),
        }
    }
}

pub struct Runner {}

impl Runner {
    async fn tick(manager: Manager, conf: &Common) {
        let date = chrono::Utc::now();
        let current_tick = conf.current_tick(date);

        let mut db = Db::new(db_connect().unwrap());

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
            let instance = DockerExploitPool::from_model(docker, exploit.clone())
                .await
                .unwrap();

            for (target_host, target_flagid) in going_to_exploit {
                let instance = instance.clone();
                tokio::spawn(async move {
                    let before = tokio::time::Instant::now();

                    let started_at = chrono::Utc::now().naive_utc();

                    let log = instance
                        .start(target_host.to_string(), target_flagid.to_string())
                        .await
                        .unwrap()
                        .wait_for_exit()
                        .await
                        .unwrap();

                    let finished_at = chrono::Utc::now().naive_utc();

                    let mut db = Db::new(db_connect().unwrap());
                    db.add_execution(&ExecutionInserter {
                        exploit_id: exploit.id.clone(),
                        output: log.output,
                        started_at,
                        finished_at,
                    });

                    // TODO add flags too
                });
            }
        }
    }

    async fn run(self, manager: Manager, conf: &config::Root) {
        let mut tick_interval = conf
            .common
            // make sure the tick has started
            .get_tick_interval(tokio::time::Duration::from_secs(1))
            .await
            .unwrap();

        let flag_regex = Regex::new(&conf.common.format).unwrap();

        loop {
            let manager = manager.clone();
            tick_interval.tick().await;

            let manager = manager.clone();
            let common = conf.common.clone();
            spawn(async move { Runner::tick(manager, &common).await });
        }
    }
}

pub async fn main(
    config: config::Root,
    manager: Manager,
    mut runner: Runner,
) -> Result<(), Report> {
    let common = &config.common;

    let docker = Docker::connect_with_local_defaults()?;

    let db = &mut db_connect()?;
    info!("Connected to database");

    // time until start
    common.sleep_until_start().await;
    assert!(chrono::Utc::now() >= common.start);
    info!("Manager woke up!");

    let time_since_start = chrono::Utc::now() - common.start;
    info!("CTF started {:?} ago", time_since_start);

    //let host = config.runner.http_server.parse()?;
    //let server = Server::new(host, runner.clone());
    //let server_handle = spawn(async move { server.run().await });

    let runner_handle = spawn(async move { runner.run(manager, &config).await });

    join_all(vec![runner_handle /*, server_handle*/]).await;

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
