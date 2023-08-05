use angrepa::config;
use async_trait::async_trait;
use rand::Rng;
use serde_json::json;
use std::collections::HashMap;

use super::{Fetcher, Service, ServiceMap, TeamService};

#[derive(Debug)]
pub struct DummyFetcher {
    pub config: config::Root,
}

#[async_trait]
impl Fetcher for DummyFetcher {
    async fn services(&self) -> Result<ServiceMap, color_eyre::Report> {
        let mut all_services = HashMap::new();
        let mut test_service = HashMap::new();

        let cur_tick_nr = self.config.common.current_tick(chrono::Utc::now()) as i32;

        self.ips().await?.into_iter().for_each(|ip| {
            let mut rng = rand::thread_rng();
            let tick_content = json! {[format!("user{}", rng.gen_range(0..=100)), format!("user{}", rng.gen_range(0..=100))]};

            let mut ticks = HashMap::new();
            ticks.insert(cur_tick_nr, vec![tick_content]);

            let ticks = TeamService{ticks: ticks};

            test_service.insert(ip, ticks);
        });

        all_services.insert(
            "testservice".to_string(),
            Service {
                teams: test_service,
            },
        );

        Ok(ServiceMap(all_services))
    }

    async fn ips(&self) -> Result<Vec<String>, color_eyre::Report> {
        Ok((1..=10).map(|i| format!("10.0.{i}.1")).collect())
    }
}
