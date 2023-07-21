use bollard::Docker;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use parking_lot::Mutex;
use reqwest::Url;
use std::{collections::HashMap, sync::Arc};

use color_eyre::{eyre::eyre, Report};
use futures::future::join_all;
use tokio::{select, spawn, time::interval};
use tracing::{debug, error, info, trace, warn};

use angrapa::schema::exploits::dsl::exploits;
use angrapa::{
    config::{self, Common},
    db_connect,
    models::ExploitModel,
};

mod exploit;
use exploit::exploit2::{
    docker::{DockerExploit, DockerExploitPool, DockerInstance},
    Exploit, ExploitInstance, RunLog,
};

use crate::server::Server;

mod server;

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

#[derive(Debug, Clone)]
pub struct ExploitHolder {
    /// a UNIQUE id
    pub id: String,
    pub enabled: bool,
    pub target: AttackTarget,
    pub exploit: Exploits,

    // stats
    pub run_logs: HashMap<i64, StampedRunLog>,
}

impl ExploitHolder {
    pub fn to_model(&self) -> ExploitModel {
        let ExploitHolder {
            id,
            enabled: running,
            target: attack_target,
            exploit,
            run_logs: _,
        } = self.clone();

        let exploit_kind = exploit.as_str();

        let attack_target = match attack_target {
            AttackTarget::Service(s) => Some(s),
            AttackTarget::Ips => None,
        };

        let docker_image = match &exploit {
            Exploits::DockerPool(pool) => pool.image.to_owned(),
            Exploits::Docker(single) => single.image.to_owned(),
        };

        ExploitModel {
            id,
            running,
            attack_target,
            docker_image,
            exploit_kind,
        }
    }
}

#[derive(Debug, Clone)]
pub enum AttackTarget {
    /// attack a specific service, runner will ask manager for flagids and ips
    Service(String),
    /// attack all ips, runner will ask manager for all ips
    /// this is useful when there is no flagid
    Ips,
}

/// RunLog with metadata
#[derive(Debug, Clone)]
pub struct StampedRunLog {
    pub tick: i64,
    // the task
    pub flagstore: Option<String>,
    pub log: RunLog,
}

#[derive(Debug, Clone)]
pub struct Runner {
    // TODO possibly wrap this in a mutex so we can access this from multiple
    // places..? channels aren't that nice when the code is this complex, and
    // we want to get the result value (i.e. error if starting a non-existant
    // exploit...)
    exploits: Arc<Mutex<HashMap<String, ExploitHolder>>>,

    /// Queue of output data to be sent to the manager (for flag submission)
    output_queue: Arc<Mutex<Vec<StampedRunLog>>>,
}

impl Runner {
    pub fn new() -> Self {
        Self {
            exploits: Arc::new(Mutex::new(HashMap::new())),
            output_queue: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn log_run(&self, id: &str, log: StampedRunLog) {
        info!("Logging run for exploit {}", id);

        // insert into queue
        {
            let mut lock = self.output_queue.lock();
            lock.push(log.clone());
        }

        // insert into log database
        {
            let mut lock = self.exploits.lock();
            let holder = lock.get_mut(id).unwrap();
            holder.run_logs.insert(log.tick, log.clone());
        }

        debug!("inserted run log for tick {} for exploit {}", log.tick, id);
    }

    async fn send_flags(&self, conf: &config::Root) {
        let output = {
            let mut lock = self.output_queue.lock();
            lock.drain(..).collect::<Vec<_>>()
        };

        if output.is_empty() {
            return;
        }

        // yank this from the *manager* config
        let base = format!("http://{host}/submit", host = conf.manager.http_listener);
        info!("Sending {} flags to {}", output.len(), base);

        let client = reqwest::Client::new();

        for stamped in output {
            let client = client.clone();

            let mut params = vec![("tick", stamped.tick.to_string())];
            if let Some(flagstore) = stamped.flagstore {
                params.push(("flagstore", flagstore));
            }

            let url = Url::parse_with_params(&base, params).unwrap();

            spawn(async move {
                debug!("sending to {}", url);
                let response = client.post(url).body(stamped.log.output).send().await;
                match &response {
                    Err(e) => warn!("error sending flag: {:?}", e),
                    Ok(r) => {
                        if !r.status().is_success() {
                            warn!("error sending flag: {:?}", r);
                        }
                    }
                }
                trace!("got response: {:?}", response);
            });
        }
    }

    /// Register without adding to DB
    async fn register_existing_exp(&mut self, exp: ExploitHolder) {
        info!("Registering exploit locally. {:?}", exp);

        let mut lock = self.exploits.lock();
        lock.insert(exp.id.clone(), exp.clone());

        info!("Registered exploit locally {}", exp.id);
    }

    async fn register_exp(&mut self, exp: ExploitHolder) {
        info!("Registering new exploit. {:?}", exp);

        // first instert into db
        let db = &mut db_connect().unwrap();
        let x: ExploitModel = diesel::insert_into(angrapa::schema::exploits::table)
            .values(exp.to_model())
            .returning(angrapa::schema::exploits::all_columns)
            .get_result(db)
            .unwrap();
        info!("Inserted exploit into database: {:?}", x);

        // then into the local exploit map
        self.register_existing_exp(exp.clone()).await;

        info!("Registered exploit {}", exp.id);
    }

    async fn tick(&self, conf: &Common) {
        let date = chrono::Utc::now();
        let current_tick = conf.current_tick(date);

        let rnr = self.clone();
        let lock = self.exploits.lock();

        info!(
            "tick {}. exploits: {}, enabled: {}, disabled: {}",
            current_tick,
            lock.len(),
            lock.iter().filter(|(_, v)| v.enabled).count(),
            lock.iter().filter(|(_, v)| !v.enabled).count(),
        );

        for (_id, holder) in lock.iter() {
            let rnr = rnr.clone();
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

                // append log
                let log = StampedRunLog {
                    tick: current_tick,
                    flagstore: None,
                    log: log,
                };
                rnr.log_run(&holder.id, log.clone()).await;

                let elapsed = before.elapsed();
                info!("Execution took {:?}, output: {:?}", elapsed, log.log.output)
            });
        }
    }

    // todo proper result type, but for now it doesnt matter
    async fn start(&mut self, id: &str) -> Result<(), Report> {
        let mut lock = self.exploits.lock();

        let holder = match lock.get_mut(id) {
            Some(holder) => holder,
            None => {
                warn!("Tried to start non-existant exploit {}", id);
                return Err(eyre!("Tried to start non-existant exploit {}", id));
            }
        };

        info!("Starting exploit {}", id);
        holder.enabled = true;
        drop(lock);

        // update db
        let db = &mut db_connect().unwrap();
        let x: ExploitModel = diesel::update(exploits.find(id))
            .set(angrapa::schema::exploits::running.eq(true))
            .get_result(db)
            .unwrap();
        debug!("Updated exploit in database: {:?}", x);

        Ok(())
    }

    async fn stop(&mut self, id: &str) -> Result<(), Report> {
        let mut lock = self.exploits.lock();

        let holder = match lock.get_mut(id) {
            Some(holder) => holder,
            None => {
                warn!("Tried to stop non-existant exploit {}", id);
                return Err(eyre!("Tried to stop non-existant exploit {}", id));
            }
        };

        info!("Stopping exploit {}", id);
        holder.enabled = false;
        drop(lock);

        // update db
        let db = &mut db_connect().unwrap();
        let x: ExploitModel = diesel::update(exploits.find(id))
            .set(angrapa::schema::exploits::running.eq(false))
            .get_result(db)
            .unwrap();
        debug!("Updated exploit in database: {:?}", x);

        Ok(())
    }

    async fn run(self, conf: &config::Root) {
        let mut tick_interval = conf
            .common
            // make sure the tick has started
            .get_tick_interval(tokio::time::Duration::from_secs(1))
            .await
            .unwrap();

        let mut flag_interval = interval(tokio::time::Duration::from_secs(1));

        loop {
            select! {
                _ = tick_interval.tick() => self.tick(&conf.common).await,
                // on another thread
                _ = flag_interval.tick() => self.send_flags(conf).await,
            }
        }
    }
}

async fn reconstruct_exploit(
    docker: &Docker,
    model: ExploitModel,
) -> Result<ExploitHolder, Report> {
    let docker_exp = DockerExploit::from_model(docker.clone(), model.clone()).await?;

    let exploit = match model.exploit_kind.as_str() {
        "docker" => Exploits::Docker(docker_exp),
        "docker_pool" => Exploits::DockerPool(docker_exp.spawn_pool().await?),
        _ => panic!("Unknown exploit kind {}", model.exploit_kind),
    };

    Ok(ExploitHolder {
        id: model.id,
        enabled: model.running,
        target: match model.attack_target {
            Some(s) => AttackTarget::Service(s),
            None => AttackTarget::Ips,
        },
        exploit,
        run_logs: HashMap::new(),
    })
}

#[tokio::main]
async fn main() -> Result<(), Report> {
    color_eyre::install()?;

    // get config
    let args = argh::from_env::<angrapa::config::Args>();
    let config = args.get_config()?;
    let common = &config.common;

    // setup logging
    args.setup_logging()?;

    let mut runner = Runner::new();
    let docker = Docker::connect_with_local_defaults()?;

    let db = &mut db_connect()?;
    info!("Connected to database");

    let exps: Vec<ExploitModel> = exploits.load(db)?;
    info!("Found {} existing exploits", exps.len());
    for model in exps {
        let exploit = reconstruct_exploit(&docker, model).await;
        let exploit = match exploit {
            Ok(exploit) => exploit,
            Err(e) => {
                error!("Error reconstructing exploit: {:?}", e);
                continue;
            }
        };

        runner.register_existing_exp(exploit).await;
    }

    // time until start
    common.sleep_until_start().await;
    assert!(chrono::Utc::now() >= common.start);
    info!("Manager woke up!");

    let time_since_start = chrono::Utc::now() - common.start;
    info!("CTF started {:?} ago", time_since_start);

    let host = config.runner.http_server.parse()?;
    let server = Server::new(host, runner.clone());
    let server_handle = spawn(async move { server.run().await });

    let runner_handle = spawn(async move { runner.run(&config).await });

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
