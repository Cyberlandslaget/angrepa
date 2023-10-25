use async_trait::async_trait;
use serde::{self, Deserialize};
use std::collections::HashMap;
use tracing::warn;

use super::{Fetcher, FetcherError, Service, ServiceMap, TeamService};

#[derive(Deserialize, Debug)]
pub struct AttackInfo {
    pub teams: Vec<i32>,
    // TODO! should also accept <i32, _> and convert the i32 to String...
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
        let client = reqwest::Client::builder()
            .pool_max_idle_per_host(0) // should disable pooling which fixes errors against some hosts
            .build()
            .unwrap();

        Self {
            client,
            teams,
            scoreboard,
            format,
        }
    }
}

fn fix_team(team: String) -> String {
    team.replace("lol_1", "129.241.150.218")
        .replace("lol_2", "129.241.150.84")
        .replace("lol_3", "129.241.150.59")
        .replace("lol_4", "129.241.150.226")
        .replace("lol_5", "129.241.150.105")
        .replace("lol_6", "129.241.150.42")
        .replace("lol_7", "129.241.150.123")
        .replace("lol_8", "129.241.150.254")
        .replace("lol_9", "129.241.150.160")
        .replace("lol_10", "129.241.150.209")
        .replace("lol_11", "129.241.150.153")
        .replace("lol_12", "129.241.150.241")
        .replace("lol_13", "129.241.150.188")
        .replace("lol_14", "129.241.150.248")
        .replace("lol_15", "129.241.150.152")
        .replace("lol_16", "129.241.150.130")
        .replace("lol_17", "129.241.150.184")
        .replace("lol_18", "129.241.150.97")
        .replace("lol_19", "129.241.150.229")
        .replace("lol_20", "129.241.150.113")
        .replace("lol_21", "129.241.150.151")
        .replace("lol_22", "129.241.150.119")
        .replace("lol_23", "129.241.150.206")
        .replace("lol_24", "129.241.150.245")
        .replace("lol_25", "129.241.150.111")
        .replace("lol_26", "129.241.150.131")
        .replace("lol_27", "129.241.150.114")
        .replace("lol_28", "129.241.150.78")
        .replace("lol_29", "129.241.150.138")
        .replace("lol_30", "129.241.150.214")
        .replace("lol_31", "129.241.150.115")
        .replace("lol_32", "129.241.150.32")
        .replace("lol_33", "129.241.150.15")
        .replace("lol_34", "129.241.150.233")
        .replace("lol_35", "129.241.150.171")
}

#[async_trait]
impl Fetcher for FaustFetcher {
    type Error = FetcherError;

    async fn services(&self) -> Result<ServiceMap, Self::Error> {
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
                let team = fix_team(self.format.replace("{x}", &format!("{}", team)));

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

    async fn ips(&self) -> Result<Vec<String>, Self::Error> {
        let resp: AttackInfo = self.client.get(&self.teams).send().await?.json().await?;

        let ips = resp
            .teams
            .into_iter()
            .map(|team_nr| fix_team(self.format.replace("{x}", &format!("{}", team_nr))))
            .collect();

        Ok(ips)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use warp::Filter;

    const TEAMS_JSON: &str = r#"
            {
                "teams": [
                    2
                ],
                "flag_ids": {
                    "service_1": {
                        "2": [
                                [
                                    [ "user73" ],
                                    [ "user5" ]
                                ],
                                [
                                    [ "user96" ],
                                    [ "user314" ]
                                ]
                        ]
                    }
                }
            }"#;

    const SCOREBOARD_JSON: &str = r#"
            {
                "current_tick": 271
            }
    "#;

    #[tokio::test]
    async fn faust_local_test() {
        let gameserver = tokio::spawn(async move {
            let teams = warp::path!("teams").map(|| TEAMS_JSON);
            let scoreboard = warp::path!("scoreboard").map(|| SCOREBOARD_JSON);
            warp::serve(teams.or(scoreboard))
                .run(([127, 0, 0, 1], 8888))
                .await
        });

        let fetcher = FaustFetcher::new(
            "http://localhost:8888/teams".to_string(),
            "http://localhost:8888/scoreboard".to_string(),
            "1.20.{x}.1".to_string(),
        );

        let services = fetcher.services().await.unwrap();

        dbg!(&services);

        gameserver.abort();
    }
}
