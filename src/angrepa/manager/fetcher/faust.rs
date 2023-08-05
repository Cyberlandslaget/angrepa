use async_trait::async_trait;
use serde::{self, Deserialize};
use std::collections::HashMap;
use tracing::warn;

use super::{Fetcher, Service, ServiceMap, TeamService};

#[derive(Deserialize, Debug)]
pub struct AttackInfo {
    pub teams: Vec<i32>,
    pub flag_ids: HashMap<String, ServiceContent>,
}

#[derive(Deserialize, Debug)]
pub struct Scoreboard {
    pub current_tick: i32,
}

/// teamid -> Vec<flagid>
#[derive(Deserialize, Debug)]
pub struct ServiceContent(HashMap<String, serde_json::Value>); // treat all the flagids as one

#[derive(Debug)]
pub struct FaustFetcher {
    client: reqwest::Client,
    teams: String,
    format: String,
    scoreboard: String,
}

impl FaustFetcher {
    pub fn new(teams: String, scoreboard: String, format: String) -> Self {
        let client = reqwest::Client::new();

        Self {
            client,
            teams,
            scoreboard,
            format,
        }
    }
}

#[async_trait]
impl Fetcher for FaustFetcher {
    async fn services(&self) -> Result<ServiceMap, color_eyre::Report> {
        let scoreboard: Scoreboard = self
            .client
            .get(&self.scoreboard)
            .send()
            .await?
            .json()
            .await?;

        let resp: AttackInfo = self.client.get(&self.teams).send().await?.json().await?;

        let mut services = HashMap::new();
        for (service, content) in resp.flag_ids {
            let mut service_content = HashMap::new();

            // shitty solution: we dont know which flagid is for which tick, so just give all the
            // current ones for the current tick\
            // the fetcher routine should discard the duplicates

            // on cold start: ex. 5 flagids sent for current tick
            // every tick afterwards: just 1 flagid, because 4 others are known

            let current_tick = scoreboard.current_tick;

            for (team, flagids) in content.0 {
                // faust gives an array of the last few flagids here, extract them manually :grimace:
                let flagids = match flagids.as_array() {
                    Some(a) => a,
                    None => {
                        warn!("Should be array but isn't");
                        continue;
                    }
                }
                .to_owned();

                let team = team.parse::<i32>().unwrap();
                let team = self.format.replace("{x}", &format!("{}", team));

                let mut ticks = HashMap::new();
                ticks.insert(current_tick, flagids); // just this one

                service_content.insert(team, TeamService { ticks });
            }
            services.insert(
                service,
                Service {
                    teams: service_content,
                },
            );
        }

        Ok(ServiceMap(services))
    }

    async fn ips(&self) -> Result<Vec<String>, color_eyre::Report> {
        let resp: AttackInfo = self.client.get(&self.teams).send().await?.json().await?;

        let ips = resp
            .teams
            .into_iter()
            .map(|team_nr| self.format.replace("{x}", &format!("{}", team_nr)))
            .collect();

        Ok(ips)
    }
}
