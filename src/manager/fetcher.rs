use std::collections::HashMap;

use angrapa::config;
use async_trait::async_trait;
use color_eyre::{eyre::eyre, Report};
use serde::{Deserialize, Serialize};

mod enowars;
pub use enowars::EnowarsFetcher;
mod dummy;
pub use dummy::DummyFetcher;
use tracing::{info, warn};

#[derive(Debug)]
pub enum Fetchers {
    Enowars(EnowarsFetcher),
    Dummy(DummyFetcher),
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Service(HashMap<String, serde_json::Value>);

/// Implements fetching flagids and hosts
#[async_trait]
pub trait Fetcher {
    async fn services(&self) -> Result<HashMap<String, Service>, Report>;
}

// routine
pub async fn run(fetcher: impl Fetcher, common: &config::Common) {
    let mut tick_interval = common
        .get_tick_interval(tokio::time::Duration::from_secs(1))
        .await
        .unwrap();

    let mut last_services = None;

    loop {
        tick_interval.tick().await;
        let tick_number = common.current_tick(chrono::Utc::now());

        let services = fetcher.services().await.unwrap();
        let service_names = services.keys().collect::<Vec<_>>();
        info!("tick {}: services: {:?}", tick_number, service_names);

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

                Ok(Self::Enowars(EnowarsFetcher::new(endpoint)))
            }
            _ => Err(eyre!("Unknown fetcher {}", manager.fetcher_name)),
        }
    }
}
