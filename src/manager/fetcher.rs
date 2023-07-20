use std::collections::HashMap;

use async_trait::async_trait;
use color_eyre::Report;
use serde::{Deserialize, Serialize};

mod enowars;

#[derive(Serialize, Deserialize, Debug)]
pub struct Service(HashMap<String, serde_json::Value>);

/// Implements fetching flagids and hosts
#[async_trait]
pub trait Fetcher {
    async fn services(&self) -> Result<HashMap<String, Service>, Report>;
}
