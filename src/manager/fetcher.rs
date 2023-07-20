use std::collections::HashMap;

use angrapa::config;
use async_trait::async_trait;
use color_eyre::{eyre::eyre, Report};
use serde::{Deserialize, Serialize};

mod enowars;
pub use enowars::EnowarsFetcher;
mod dummy;
pub use dummy::DummyFetcher;

#[derive(Debug)]
pub enum Fetchers {
    Enowars(EnowarsFetcher),
    Dummy(DummyFetcher),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Service(HashMap<String, serde_json::Value>);

/// Implements fetching flagids and hosts
#[async_trait]
pub trait Fetcher {
    async fn services(&self) -> Result<HashMap<String, Service>, Report>;
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
