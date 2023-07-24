use std::collections::HashMap;

use async_trait::async_trait;
use color_eyre::{eyre::eyre, Report};
use serde::{Deserialize, Serialize};

use super::CONFIG;

mod enowars;
pub use enowars::EnowarsFetcher;
mod dummy;
pub use dummy::DummyFetcher;
use tracing::{info, warn};

use super::Manager;

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
pub async fn run(fetcher: impl Fetcher, manager: Manager) {
    let common = &CONFIG.common;
    let mut tick_interval = common
        .get_tick_interval(tokio::time::Duration::from_secs(1))
        .await
        .unwrap();

    let mut last_services = None;

    loop {
        // wait for new tick
        tick_interval.tick().await;
        let tick_number = common.current_tick(chrono::Utc::now());

        // get updated info
        let services = fetcher.services().await.unwrap();
        let service_names = services.keys().collect::<Vec<_>>();
        info!("tick {}: services: {:?}", tick_number, service_names);

        let ips = fetcher.ips().await.unwrap();

        // then save it
        manager.update_ips_services(tick_number as i32, ips, services.clone());

        // some checks
        if let Some(last) = last_services {
            if last == services {
                // something is wrong, the flagids did not update!
                warn!("Got the same services as last time");
            }
        }

        last_services = Some(services);
    }
}

// Deserialize
impl Fetchers {
    pub fn from_conf() -> Result<Self, Report> {
        let manager = &CONFIG.manager;
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
