use bollard::Docker;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use parking_lot::Mutex;
use regex::Regex;
use std::{collections::HashMap, sync::Arc};

use color_eyre::{eyre::eyre, Report};
use futures::future::join_all;
use tokio::{select, spawn, time::interval};
use tracing::{debug, error, info, warn};

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

mod server;
use server::Server;

use crate::manager::{Flag, Manager};

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
            blacklist: vec![],
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
    // data
    pub log: RunLog,
    // metadata
    pub tick: i64,
    pub flagstore: String,
    pub target_ip: String,
    pub exploit_id: String,
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
            info!(
                "Inserted into output_queue, now has {} elements",
                lock.len()
            );
        }

        // insert into log database
        {
            let mut lock = self.exploits.lock();
            let holder = lock.get_mut(id).unwrap();
            holder.run_logs.insert(log.tick, log.clone());
        }

        debug!("inserted run log for tick {} for exploit {}", log.tick, id);
    }

    async fn send_flags(&self, manager: Manager, regex: &Regex) {
        let output = {
            let mut lock = self.output_queue.lock();
            lock.drain(..).collect::<Vec<_>>()
        };

        if output.is_empty() {
            return;
        }

        for StampedRunLog {
            tick,
            flagstore,
            log,
            target_ip,
            exploit_id,
        } in output
        {
            for flag in regex.captures_iter(&log.output) {
                let flag = flag[0].to_string();

                let tick = Some(tick as i32);
                let stamp = Some(chrono::Utc::now().naive_utc());
                let flagstore = Some(flagstore.clone());
                let target_ip = Some(target_ip.clone());
                let exploit_id = Some(exploit_id.clone());

                info!("Runner is registering flag directly on manager: {}", flag);

                manager.register_flag(Flag {
                    flag,
                    tick,
                    stamp,
                    exploit_id,
                    target_ip,
                    flagstore,
                    sent: false,
                    status: None,
                });
            }
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

    async fn tick(&self, manager: Manager, conf: &Common) {
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

        for (_id, holder) in lock
            .iter()
            // only enabled exploits
            .filter(|(_, v)| v.enabled)
        {
            info!("Attacking target '{:?}'", holder.target);
            let flagstore = match &holder.target {
                AttackTarget::Service(s) => s,
                AttackTarget::Ips => "", // this service shouldnt exist
            };
            let targets = manager.get_service_target(&flagstore);

            for (target_host, target_flagid) in targets {
                let rnr = rnr.clone();
                let holder = holder.clone();

                // empty string if no flagid
                let target_flagid = target_flagid.unwrap_or_default();

                let flagstore = flagstore.to_owned();

                tokio::spawn(async move {
                    let before = tokio::time::Instant::now();
                    let log = match holder.exploit {
                        Exploits::DockerPool(pool) => {
                            let inst = pool
                                .start(target_host.to_string(), target_flagid.to_string())
                                .await
                                .unwrap();
                            inst.wait_for_exit().await.unwrap()
                        }
                        Exploits::Docker(single) => {
                            let inst = single
                                .start(target_host.to_string(), target_flagid.to_string())
                                .await
                                .unwrap();
                            inst.wait_for_exit().await.unwrap()
                        }
                    };

                    // append log
                    let log = StampedRunLog {
                        tick: current_tick,
                        flagstore,
                        log: log,
                        target_ip: target_host,
                        exploit_id: holder.id.clone(),
                    };
                    rnr.log_run(&holder.id, log.clone()).await;

                    let elapsed = before.elapsed();
                    info!("Execution took {:?}, output: {:?}", elapsed, log.log.output)
                });
            }
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

    async fn run(self, manager: Manager, conf: &config::Root) {
        let mut tick_interval = conf
            .common
            // make sure the tick has started
            .get_tick_interval(tokio::time::Duration::from_secs(1))
            .await
            .unwrap();

        let flag_regex = Regex::new(&conf.common.format).unwrap();

        let mut flag_interval = interval(tokio::time::Duration::from_secs(1));

        loop {
            let manager = manager.clone();
            let r = self.clone();
            select! {
                _ = tick_interval.tick() => {
                    let manager = manager.clone();
                    let common = conf.common.clone();
                    spawn(async move {
                        r.tick(manager, &common).await
                    });
                },
                _ = flag_interval.tick() => {
                    let manager = manager.clone();
                    let flag_regex = flag_regex.clone();
                    spawn(async move {
                        r.send_flags(manager, &flag_regex).await
                    });
                },
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

pub async fn main(
    config: config::Root,
    manager: Manager,
    mut runner: Runner,
) -> Result<(), Report> {
    let common = &config.common;

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

    let runner_handle = spawn(async move { runner.run(manager, &config).await });

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
