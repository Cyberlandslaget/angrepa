use async_trait::async_trait;
use rand::Rng;
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

    async fn ips(&self) -> Result<Vec<String>, color_eyre::Report> {
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

        Ok((0..5).map(|_| random_ip()).collect())
    }
}
