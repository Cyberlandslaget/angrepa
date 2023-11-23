use async_trait::async_trait;
use std::collections::HashMap;

use super::{Fetcher, FetcherError, ServiceMap};

#[derive(Debug)]
pub struct StatiskFetcher {
    pub ids: Vec<u8>,
}

#[async_trait]
impl Fetcher for StatiskFetcher {
    async fn services(&self) -> Result<ServiceMap, FetcherError> {
        Ok(ServiceMap(HashMap::new()))
    }

    async fn ips(&self) -> Result<Vec<String>, FetcherError> {
        Ok(self.ids.iter().map(|i| format!("10.10.{i}.2")).collect())
    }
}
