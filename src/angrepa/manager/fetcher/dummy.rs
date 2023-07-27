use angrepa::config;
use async_trait::async_trait;
use rand::Rng;
use serde_json::json;
use std::collections::HashMap;

use super::{Fetcher, Service, Ticks};

#[derive(Debug)]
pub struct DummyFetcher {
    pub config: config::Root,
}

#[async_trait]
impl Fetcher for DummyFetcher {
    async fn services(&self) -> Result<HashMap<String, Service>, color_eyre::Report> {
        let mut map = HashMap::new();
        let mut test_service = HashMap::new();

        self.ips().await?.into_iter().for_each(|ip| {
            let mut rng = rand::thread_rng();
            let test_tick = json! {[format!("user{}", rng.gen_range(0..=100)), format!("user{}", rng.gen_range(0..=100))]};

            let mut ticks = HashMap::new();
            let cur_tick = self.config.common.current_tick(chrono::Utc::now()) as i32;
            ticks.insert(cur_tick, test_tick);

            let ticks = Ticks(ticks);

            test_service.insert(ip, ticks);

        });

        map.insert("testservice".to_string(), Service(test_service));

        Ok(map)
    }

    async fn ips(&self) -> Result<Vec<String>, color_eyre::Report> {
        Ok((1..=10).map(|i| format!("10.0.{i}.1")).collect())
    }
}
