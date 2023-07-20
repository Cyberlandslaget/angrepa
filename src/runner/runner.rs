use angrapa::config::Common;
use color_eyre::eyre;
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

struct Runner {
    exploits: Vec<ExploitHolder>,
    exploit_rx: flume::Receiver<ExploitHolder>,
}

impl Runner {
    async fn register_exp(&mut self, exp: ExploitHolder) {
        info!("Registering new exploit. {:?}", exp);
        self.exploits.push(exp);
    }

    async fn tick(&self, conf: &Common) {
        let date = chrono::Utc::now();
        let current_tick = conf.current_tick(date);
        info!(
            "tick {} (UTC {})",
            current_tick,
            date.format("%Y-%m-%d %H:%M:%S.%f")
        );

        for holder in &self.exploits {
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

    async fn run(mut self, conf: &Common) {
        let mut interval = conf
            // make sure the tick has started
            .get_tick_interval(tokio::time::Duration::from_secs(1))
            .await
            .unwrap();

        loop {
            select! {
                _ = interval.tick() => self.tick(conf).await,
                exp = self.exploit_rx.recv_async() => {
                    match exp {
                        Ok(exp) => self.register_exp(exp).await,
                        Err(err) => warn!("Failed to recv exploit: {:?}", err),
                    }
                },
            };
        }
    }
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
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

    let tar = tarify("data/exploits/new")?;
    let docker = DockerInstance::new()?;

    let (exploit_tx, exploit_rx) = flume::unbounded();

    // keep original around, otherwise closed errors
    let exploit_tx2 = exploit_tx.clone();
    spawn(async move {
        let exploit = docker.new_exploit(&tar).await.unwrap();
        let pool = exploit.spawn_pool().await.unwrap();

        exploit_tx
            .send_async(ExploitHolder {
                enabled: false,
                target: AttackTarget::Ips,
                exploit: Exploits::Docker(exploit),
            })
            .await
            .unwrap();

        exploit_tx
            .send_async(ExploitHolder {
                enabled: false,
                target: AttackTarget::Ips,
                exploit: Exploits::DockerPool(pool),
            })
            .await
            .unwrap();
    });

    let runner = Runner {
        exploits: Vec::new(),
        exploit_rx,
    };

    let runner_handle = spawn(async move { runner.run(&common).await });

    let host = config.runner.http_server.parse()?;
    let server = Server::new(host, exploit_tx2);
    let server_handle = spawn(async move { server.run().await });

    join_all(vec![runner_handle, server_handle]).await;

    Ok(())
}

fn tarify(path: &str) -> eyre::Result<Vec<u8>> {
    use tar::Builder;

    let mut tar = Builder::new(Vec::new());

    tar.append_dir_all(".", path)?;
    tar.finish()?;

    let tar = tar.into_inner()?;
    Ok(tar)
}
