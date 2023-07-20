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
}

impl EnowarsFetcher {
    pub fn new(endpoint: String) -> Self {
        let client = reqwest::Client::new();

        Self { client, endpoint }
    }
}

#[async_trait]
impl Fetcher for EnowarsFetcher {
    async fn services(&self) -> Result<HashMap<String, Service>, color_eyre::Report> {
        let resp: AttackInfo = self.client.get(&self.endpoint).send().await?.json().await?;

        Ok(resp.services)
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

        let services = attack_info.services;

        services
    }

    #[tokio::test]
    /// Fetch the response from a local test server
    async fn eno_local_test() {
        let gameserver = tokio::spawn(async move {
            // note, content-type not set probably
            let endpoint = warp::path!("endpoint").map(|| JSON);

            warp::serve(endpoint).run(([127, 0, 0, 1], 9999)).await
        });

        let fetcher = EnowarsFetcher::new("http://localhost:9999/endpoint".to_string());

        let services = fetcher.services().await.unwrap();

        // make sure we got the same content as directly deserializing locally
        assert_eq!(
            serde_json::to_string(&services).unwrap(),
            serde_json::to_string(&eno_deser()).unwrap()
        );

        gameserver.abort();
    }
}
