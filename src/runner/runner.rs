use parking_lot::Mutex;
use std::{collections::HashMap, sync::Arc};

use angrapa::config::Common;
use color_eyre::{eyre::eyre, Report};
use futures::future::join_all;
use tokio::{select, spawn};
use tracing::{info, warn};

mod exploit;
use exploit::exploit2::{
    docker::{DockerExploit, DockerExploitPool, DockerInstance},
    Exploit, ExploitInstance,
};

use crate::server::Server;

mod server;

#[derive(Debug, Clone)]
pub enum Exploits {
    DockerPool(DockerExploitPool),
    Docker(DockerExploit),
}

#[derive(Debug, Clone)]
pub struct ExploitHolder {
    /// a UNIQUE id
    pub id: String,
    pub enabled: bool,
    pub target: AttackTarget,
    pub exploit: Exploits,
}

#[derive(Debug, Clone)]
pub enum AttackTarget {
    /// attack a specific service, runner will ask manager for flagids and ips
    Service(String),
    /// attack all ips, runner will ask manager for all ips
    /// this is useful when there is no flagid
    Ips,
}

pub enum RunnerRequest {
    Start(String),
    Stop(String),
}

#[derive(Debug, Clone)]
pub struct Runner {
    // TODO possibly wrap this in a mutex so we can access this from multiple
    // places..? channels aren't that nice when the code is this complex, and
    // we want to get the result value (i.e. error if starting a non-existant
    // exploit...)
    exploits: Arc<Mutex<HashMap<String, ExploitHolder>>>,
}

impl Runner {
    pub fn new() -> Self {
        Self {
            exploits: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn register_exp(&mut self, exp: ExploitHolder) {
        info!("Registering new exploit. {:?}", exp);
        let mut lock = self.exploits.lock();
        lock.insert(exp.id.clone(), exp);
    }

    async fn tick(&self, conf: &Common) {
        let date = chrono::Utc::now();
        let current_tick = conf.current_tick(date);

        let lock = self.exploits.lock();

        info!(
            "tick {}. exploits: {}, enabled: {}, disabled: {}",
            current_tick,
            lock.len(),
            lock.iter().filter(|(_, v)| v.enabled).count(),
            lock.iter().filter(|(_, v)| !v.enabled).count(),
        );

        for (_id, holder) in lock.iter() {
            let holder = holder.clone();
            tokio::spawn(async move {
                let before = tokio::time::Instant::now();
                let log = match holder.exploit {
                    Exploits::DockerPool(pool) => {
                        let inst = pool
                            .start("1.2.3.4".to_string(), "fakeid".to_string())
                            .await
                            .unwrap();
                        inst.wait_for_exit().await.unwrap()
                    }
                    Exploits::Docker(single) => {
                        let inst = single
                            .start("1.2.3.4".to_string(), "fakeid".to_string())
                            .await
                            .unwrap();
                        inst.wait_for_exit().await.unwrap()
                    }
                };
                let elapsed = before.elapsed();
                info!("Execution took {:?}, output: {:?}", elapsed, log.output)
            });
        }
    }

    // todo proper result type, but for now it doesnt matter
    async fn start(&mut self, id: &str) -> Result<(), Report> {
        let mut lock = self.exploits.lock();
        if let Some(holder) = lock.get_mut(id) {
            holder.enabled = true;
            info!("Starting exploit {}", id);
            Ok(())
        } else {
            warn!("Tried to start non-existant exploit {}", id);
            Err(eyre!("Tried to start non-existant exploit {}", id))
        }
    }

    async fn stop(&mut self, id: &str) -> Result<(), Report> {
        let mut lock = self.exploits.lock();
        if let Some(holder) = lock.get_mut(id) {
            holder.enabled = false;
            info!("Stopping exploit {}", id);
            Ok(())
        } else {
            warn!("Tried to stop non-existant exploit {}", id);
            Err(eyre!("Tried to stop non-existant exploit {}", id))
        }
    }

    async fn run(self, conf: &Common) {
        let mut interval = conf
            // make sure the tick has started
            .get_tick_interval(tokio::time::Duration::from_secs(1))
            .await
            .unwrap();

        loop {
            select! {
                _ = interval.tick() => self.tick(conf).await,
            };
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Report> {
    color_eyre::install()?;

    // get config
    let args = argh::from_env::<angrapa::config::Args>();
    let config = args.get_config()?;
    let common = config.common;

    // setup logging
    args.setup_logging()?;

    // time until start
    common.sleep_until_start().await;
    assert!(chrono::Utc::now() >= common.start);
    info!("Manager woke up!");

    let time_since_start = chrono::Utc::now() - common.start;
    info!("CTF started {:?} ago", time_since_start);

    // keep original around, otherwise closed errors
    //spawn(async move {
    //    let tar = tarify("data/exploits/new")?;
    //    let docker = DockerInstance::new()?;

    //    let exploit = docker.new_exploit(&tar).await.unwrap();
    //    let pool = exploit.spawn_pool().await.unwrap();

    //    exploit_tx
    //        .send_async(ExploitHolder {
    //            id: "test1".to_string(),
    //            enabled: false,
    //            target: AttackTarget::Ips,
    //            exploit: Exploits::Docker(exploit),
    //        })
    //        .await
    //        .unwrap();

    //    exploit_tx
    //        .send_async(ExploitHolder {
    //            id: "test2".to_string(),
    //            enabled: false,
    //            target: AttackTarget::Ips,
    //            exploit: Exploits::DockerPool(pool),
    //        })
    //        .await
    //        .unwrap();
    //});

    let runner = Runner::new();
    let runner2 = runner.clone();

    let runner_handle = spawn(async move { runner.run(&common).await });

    let host = config.runner.http_server.parse()?;
    let server = Server::new(host, runner2);
    let server_handle = spawn(async move { server.run().await });

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
