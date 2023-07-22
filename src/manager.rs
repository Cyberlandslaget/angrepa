use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use angrapa::config;
use angrapa::schema::flags::dsl::flags;
use angrapa::{db_connect, models::FlagModel};
use color_eyre::Report;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use futures::future::join_all;
use parking_lot::Mutex;
use regex::Regex;
use tracing::{debug, error, info};

mod submitter;
use submitter::{FlagStatus, Submitters};

mod listener;
use listener::{Tcp, Web};

use crate::runner::Runner;

use self::fetcher::Service;

mod handler;

mod fetcher;

#[derive(Clone, Debug, Default)]
pub struct Flag {
    pub flag: String,
    pub tick: Option<i32>,
    pub stamp: Option<chrono::NaiveDateTime>,
    pub exploit_id: Option<String>,
    pub target_ip: Option<String>,
    pub flagstore: Option<String>,
    pub sent: bool,
    pub status: Option<FlagStatus>,
}

impl Flag {
    pub fn from_model(model: FlagModel) -> Self {
        let FlagModel {
            flag,
            tick,
            stamp,
            exploit_id,
            target_ip,
            flagstore,
            sent,
            status,
        } = model;

        let status = if let Some(status_str) = status {
            let status = FlagStatus::from_str(&status_str);
            match status {
                Ok(status) => Some(status),
                Err(e) => {
                    error!("Error parsing status: {}", e);
                    None
                }
            }
        } else {
            None
        };

        Self {
            flag,
            tick,
            stamp,
            exploit_id,
            target_ip,
            flagstore,
            sent,
            status,
        }
    }

    pub fn to_model(self) -> FlagModel {
        let Flag {
            flag,
            tick,
            stamp,
            exploit_id,
            target_ip,
            flagstore,
            sent,
            status,
        } = self;

        let status = status.map(|s| s.to_string());

        FlagModel {
            flag,
            tick,
            stamp,
            exploit_id,
            target_ip,
            flagstore,
            sent,
            status,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Manager {
    flags: Arc<Mutex<HashMap<String, Flag>>>,
    /// raw ips
    ips: Arc<Mutex<Vec<String>>>,
    /// raw services
    services: Arc<Mutex<HashMap<String, Service>>>,
    /// last updated
    services_ips_last_tick: Arc<Mutex<Option<i32>>>,

    flag_queue: Arc<Mutex<Vec<Flag>>>,
}

impl Manager {
    pub fn from_db() -> Result<Self, Report> {
        let db = &mut db_connect()?;

        let all_flags: Vec<FlagModel> = flags.load(db)?;

        let mut flag_map = HashMap::new();
        for flag in all_flags {
            let flag = Flag::from_model(flag);
            flag_map.insert(flag.flag.clone(), flag);
        }

        info!("Loaded {} flags from db", flag_map.len());

        Ok(Self {
            flags: Arc::new(Mutex::new(flag_map)),
            ips: Arc::new(Mutex::new(Vec::new())),
            services: Arc::new(Mutex::new(HashMap::new())),
            services_ips_last_tick: Arc::new(Mutex::new(None)),
            flag_queue: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Register a new flag, will discard duplicated flag. Returns true if flag was new
    pub fn register_flag(&self, flag: Flag) -> bool {
        let mut lock = self.flags.lock();

        if lock.contains_key(&flag.flag) {
            info!("Flag {} already registered", flag.flag);
            return false;
        }

        // insert locally
        lock.insert(flag.flag.clone(), flag.clone());
        drop(lock);

        // insert into queue
        let mut lock = self.flag_queue.lock();
        lock.push(flag.clone());
        drop(lock);

        // insert into db (should rly be done first but im lazy)
        let db = &mut db_connect().unwrap();
        let res = diesel::insert_into(angrapa::schema::flags::table)
            .values(&flag.to_model())
            .get_result::<FlagModel>(db);
        let res = res.unwrap();

        debug!("Inserted flag {:?} into db", res);

        true
    }

    /// Update the status of a flag
    pub fn update_flag_status(&self, flag_flag: &str, new_status: FlagStatus) {
        let mut lock = self.flags.lock();

        debug!("Updating flag {} to {}", flag_flag, new_status);

        let flag = lock.get(flag_flag);
        let flag = match flag {
            Some(flag) => flag,
            None => {
                error!("Flag {} not found", flag_flag);
                return;
            }
        };

        // update locally
        let mut flag = flag.clone();
        flag.status = Some(new_status);
        lock.insert(flag_flag.to_string(), flag.clone());
        drop(lock);

        // update in db
        let db = &mut db_connect().unwrap();
        let _f: FlagModel = diesel::update(flags.find(flag_flag))
            .set(angrapa::schema::flags::status.eq(new_status.to_string()))
            .get_result(db)
            .unwrap();
    }

    /// Update ips and services
    pub fn update_ips_services(
        &self,
        tick: i32,
        ips: Vec<String>,
        services: HashMap<String, Service>,
    ) {
        let mut lock = self.ips.lock();
        *lock = ips;
        drop(lock);

        let mut lock = self.services.lock();
        *lock = services;
        drop(lock);

        let mut lock = self.services_ips_last_tick.lock();
        *lock = Some(tick);
        drop(lock);
    }

    /// Get ips and maybe flagids
    /// If no data yet, empty vec
    pub fn get_service_target(&self, service_str: &str) -> Vec<(String, Option<String>)> {
        let lock = self.services.lock();
        let service = lock.get(service_str);

        if let Some(service) = service {
            // this service exists

            let mut out = Vec::new();
            for (ip, flagid) in service.0.iter() {
                // from json::Value to String
                let flagid = serde_json::to_string(flagid).unwrap();

                out.push((ip.clone(), Some(flagid)));
            }

            debug!(
                "Service '{}' found, returning {} targets",
                service_str,
                out.len()
            );
            return out;
        }
        drop(lock);

        // cant find service, just return ips
        let ips = self.ips.lock().clone();
        debug!(
            "Service '{}' NOT found, returning {} ips",
            service_str,
            ips.len()
        );
        ips.into_iter().map(|ip| (ip, None)).collect()
    }
}

pub async fn main(config: config::Root, manager: Manager, _runner: Runner) -> Result<(), Report> {
    let flag_regex = Regex::new(&config.common.format)?;

    info!("manager started");

    let sub = Submitters::from_conf(&config.manager)?;
    let fetch = fetcher::Fetchers::from_conf(&config.manager)?;

    // set up channels
    let (raw_flag_tx, raw_flag_rx) = flume::unbounded::<String>();

    // run tcp listener on another thread
    let tcp_listener = {
        let flag_tx = raw_flag_tx.clone();

        let host = config.manager.tcp_listener.parse()?;
        let tcp = Tcp::new(host);

        info!("tcp listener starting on {}:{}", host.ip(), host.port());

        tokio::spawn(async move {
            tcp.run(flag_tx).await.unwrap();
        })
    };

    // run web listener on another thread
    let http_listener = {
        let flag_tx = raw_flag_tx.clone();

        let host = config.manager.http_listener.parse()?;
        let web = Web::new(host);

        info!("http listener starting on {}:{}", host.ip(), host.port());

        tokio::spawn(async move {
            web.run(flag_tx).await.unwrap();
        })
    };

    // run submitter on another thread
    let manager2 = manager.clone();
    let handler_handle = tokio::spawn(async move {
        info!("handler starting");

        match sub {
            Submitters::Dummy(submitter) => {
                handler::run(manager, raw_flag_rx, submitter, flag_regex).await;
            }
            Submitters::Faust(submitter) => {
                handler::run(manager, raw_flag_rx, submitter, flag_regex).await;
            }
        }
    });

    // run fetcher on another thread
    let fetcher_handle = tokio::spawn(async move {
        info!("fetcher starting");

        match fetch {
            fetcher::Fetchers::Enowars(fetcher) => {
                fetcher::run(fetcher, manager2, &config.common).await
            }
            fetcher::Fetchers::Dummy(fetcher) => {
                fetcher::run(fetcher, manager2, &config.common).await
            }
        };
    });

    // join all
    join_all(vec![
        tcp_listener,
        http_listener,
        handler_handle,
        fetcher_handle,
    ])
    .await;

    Ok(())
}
