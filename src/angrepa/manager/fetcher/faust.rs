use async_trait::async_trait;
use serde::{self, Deserialize};
use std::collections::HashMap;

use super::{Fetcher, Service};

#[derive(Deserialize, Debug)]
pub struct AttackInfo {
    pub teams: Vec<String>,
    pub flag_ids: HashMap<String, Service>,
}

#[derive(Debug)]
pub struct FaustFetcher {
    client: reqwest::Client,
    teams: String,
    format: String,
}

impl FaustFetcher {
    pub fn new(teams: String, format: String) -> Self {
        let client = reqwest::Client::new();

        Self {
            client,
            teams,
            format,
        }
    }
}

#[async_trait]
impl Fetcher for FaustFetcher {
    async fn services(&self) -> Result<HashMap<String, Service>, color_eyre::Report> {
        let resp: AttackInfo = self.client.get(&self.teams).send().await?.json().await?;

        Ok(resp.flag_ids)
    }

    async fn ips(&self) -> Result<Vec<String>, color_eyre::Report> {
        let resp: AttackInfo = self.client.get(&self.teams).send().await?.json().await?;

        let ips = resp
            .teams
            .into_iter()
            .map(|team_nr| self.format.replace("{x}", &team_nr))
            .collect();

        Ok(ips)
    }
}
