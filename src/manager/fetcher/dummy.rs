use async_trait::async_trait;
use rand::Rng;
use serde::{self};
use serde_json::json;
use std::collections::HashMap;

use super::{Fetcher, Service};

#[derive(Debug)]
pub struct DummyFetcher {}

#[async_trait]
impl Fetcher for DummyFetcher {
    async fn services(&self) -> Result<HashMap<String, Service>, color_eyre::Report> {
        // do it with an iterator and collect

        let random_ip = || {
            let mut rng = rand::thread_rng();
            format!(
                "{}.{}.{}.{}",
                rng.gen_range(0..255),
                rng.gen_range(0..255),
                rng.gen_range(0..255),
                rng.gen_range(0..255)
            )
        };

        let test_service = [(
            random_ip(),
            json!({
                "5": [
                    ["user49"],
                    ["user20"],
                ],
                "9": [
                    ["admin55"],
                    ["admin2"],
                ],
            }),
        )]
        .into_iter()
        .collect::<HashMap<_, _>>();
        let test_service = Service(test_service);

        let mut map = HashMap::new();
        map.insert("testservice".to_string(), test_service);

        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use crate::fetcher::{enowars::AttackInfo, EnowarsFetcher};

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
