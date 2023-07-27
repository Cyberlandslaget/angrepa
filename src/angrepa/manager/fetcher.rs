use std::collections::{HashMap, HashSet};

use angrepa::db::Db;
use angrepa::get_connection_pool;
use angrepa::{config, models::TargetInserter};
use async_trait::async_trait;
use color_eyre::{eyre::eyre, Report};
use serde::{Deserialize, Serialize};

mod enowars;
pub use enowars::EnowarsFetcher;
mod dummy;
pub use dummy::DummyFetcher;
use tracing::{error, info, warn};

#[derive(Debug)]
pub enum Fetchers {
    Enowars(EnowarsFetcher),
    Dummy(DummyFetcher),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Service(pub HashMap<String, Ticks>);

impl Service {
    #[allow(dead_code)]
    pub fn get_ticks_from_host(&self, host: &str) -> Option<&Ticks> {
        self.0.get(host)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Ticks(pub HashMap<i32, serde_json::Value>);

impl Ticks {
    /// Gets the highest tick
    #[allow(dead_code)]
    pub fn get_latest(&self) -> Option<(i32, &serde_json::Value)> {
        // gets the value of the highest key
        self.0.iter().max_by_key(|(k, _)| **k).map(|(k, v)| (*k, v))
    }
}

/// Implements fetching flagids and hosts
#[async_trait]
pub trait Fetcher {
    /// services (with flagids)
    async fn services(&self) -> Result<HashMap<String, Service>, Report>;
    /// "backup" raw get all ips
    async fn ips(&self) -> Result<Vec<String>, Report>;
}

// routine
pub async fn run(fetcher: impl Fetcher, config: &config::Root) {
    let common = &config.common;

    let mut tick_interval = common
        .get_tick_interval(tokio::time::Duration::from_secs(1))
        .await
        .unwrap();

    let db_url = config.database.url();
    let db_pool = match get_connection_pool(&db_url) {
        Ok(db) => db,
        Err(e) => return warn!("Could not acquire a database pool: {e}"),
    };

    let conn = &mut match db_pool.get() {
        Ok(conn) => conn,
        Err(e) => return warn!("Could not acquire a database connection: {}", e),
    };

    let mut db = Db::new(conn);

    fetcher.ips().await.unwrap().into_iter().for_each(|ip| {
        if let Err(e) = db.add_team_checked(&ip) {
            warn!("Failed to add team: '{ip}'. Error: {}", e);
        }
    });

    loop {
        // wait for new tick
        tick_interval.tick().await;
        let tick_number = common.current_tick(chrono::Utc::now());

        // get updated info
        let services = fetcher.services().await.unwrap();
        let service_names = services.keys().cloned().collect::<HashSet<_>>();

        if service_names != common.services {
            error!(
                "Fetcher and config disagree on service names! {:?} != {:?}",
                service_names, common.services
            );
            continue;
        }

        info!("tick {}", tick_number);

        for (service_name, service) in &services {
            for (team_ip, ticks) in &service.0 {
                #[allow(clippy::for_kv_map)]
                for (tick, flag_id) in &ticks.0 {
                    // TODO check if (service_name, team_ip, tick) exists, otherwise add new flagid

                    let exists = false;

                    if !exists {
                        let inserter = TargetInserter {
                            flag_id: flag_id.to_string(),
                            service: service_name.to_owned(),
                            team: team_ip.to_owned(),
                            created_at: chrono::Utc::now().naive_utc(),
                            target_tick: *tick,
                        };

                        let conn = &mut match db_pool.get() {
                            Ok(conn) => conn,
                            Err(e) => {
                                error!("Could not acquire a database connection: {}", e);
                                continue;
                            }
                        };

                        let mut db = Db::new(conn);

                        match db.add_target(&inserter) {
                            Ok(_) => (),
                            Err(e) => {
                                error!("Could not add target: {}", e);
                                continue;
                            }
                        }
                    }
                }
            }
        }
    }
}

// Deserialize
impl Fetchers {
    pub fn from_conf(manager: &config::Manager) -> Result<Self, Report> {
        match manager.fetcher_name.as_str() {
            "dummy" => Ok(Self::Dummy(DummyFetcher {})),
            "enowars" => {
                let endpoint = manager
                    .fetcher
                    .get("endpoint")
                    .ok_or(eyre!("Enowars fetcher requires endpoint"))?
                    .as_str()
                    .ok_or(eyre!("Enowars fetcher endpoint must be a string"))?
                    .to_owned();

                let ips_endpoint = manager
                    .fetcher
                    .get("ips")
                    .ok_or(eyre!("Enowars fetcher requires ip endpoint"))?
                    .as_str()
                    .ok_or(eyre!("Enowars fetcher endpoint must be a string"))?
                    .to_owned();

                Ok(Self::Enowars(EnowarsFetcher::new(endpoint, ips_endpoint)))
            }
            _ => Err(eyre!("Unknown fetcher {}", manager.fetcher_name)),
        }
    }
}
