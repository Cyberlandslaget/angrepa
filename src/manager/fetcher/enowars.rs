use async_trait::async_trait;
use serde::{self, Deserialize};
use std::collections::HashMap;

use super::{Fetcher, Service};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AttackInfo {
    #[allow(dead_code)]
    pub available_teams: Vec<String>,
    pub services: HashMap<String, Service>,
}

#[derive(Debug)]
pub struct EnowarsFetcher {
    client: reqwest::Client,
    endpoint: String,
    ips_endpoint: String,
}

impl EnowarsFetcher {
    pub fn new(endpoint: String, ips_endpoint: String) -> Self {
        let client = reqwest::Client::new();

        Self {
            client,
            endpoint,
            ips_endpoint,
        }
    }
}

#[async_trait]
impl Fetcher for EnowarsFetcher {
    async fn services(&self) -> Result<HashMap<String, Service>, color_eyre::Report> {
        // TODO handle failures more gracefully (retry?)
        let resp: AttackInfo = self.client.get(&self.endpoint).send().await?.json().await?;

        Ok(resp.services)
    }

    async fn ips(&self) -> Result<Vec<String>, color_eyre::Report> {
        let resp: String = self
            .client
            .get(&self.ips_endpoint)
            .send()
            .await?
            .text()
            .await?;

        let ips = resp.trim().lines().map(|s| s.trim().to_string()).collect();

        Ok(ips)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use warp::Filter;

    // from https://7.enowars.com/setup
    const JSON: &str = r#"
            {
                "availableTeams": [
                    "10.1.52.1"
                ],
                "services": {
                    "service_1": {
                        "10.1.52.1": {
                            "7": [
                                [ "user73" ],
                                [ "user5" ]
                            ],
                            "8": [
                                [ "user96" ],
                                [ "user314" ]
                            ]
                        }
                    }
                }
            }"#;

    fn eno_deser() -> HashMap<String, Service> {
        let attack_info: AttackInfo = serde_json::from_str(JSON).unwrap();

        attack_info.services
    }

    #[tokio::test]
    /// Fetch the response from a local test server
    async fn eno_local_test() {
        let gameserver = tokio::spawn(async move {
            // note, content-type not set probably
            let endpoint = warp::path!("endpoint").map(|| JSON);

            warp::serve(endpoint).run(([127, 0, 0, 1], 9999)).await
        });

        let fetcher =
            EnowarsFetcher::new("http://localhost:9999/endpoint".to_string(), "".to_string());

        let services = fetcher.services().await.unwrap();

        dbg!(&services);

        for (service, service_info) in services.iter() {
            for (ip, ticks) in service_info.0.iter() {
                for (tick, flagids) in ticks.0.iter() {
                    println!("{} {} {} {}", service, ip, tick, flagids);
                }
            }
        }

        // make sure we got the same content as directly deserializing locally
        assert_eq!(
            serde_json::to_string(&services).unwrap(),
            serde_json::to_string(&eno_deser()).unwrap()
        );

        gameserver.abort();
    }
}
