use super::{FlagStatus, SubmitError, Submitter};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::{collections::HashMap, time::Instant};
use tracing::{debug, trace, warn};

#[derive(Clone, Debug)]
pub struct DctfSubmitter {
    client: reqwest::Client,
    url: String,
    cookie: String,
}

impl DctfSubmitter {
    pub fn new(url: String, cookie: String) -> Self {
        let client = reqwest::Client::builder()
            .pool_max_idle_per_host(0)
            .build()
            .unwrap();

        Self {
            client,
            url,
            cookie,
        }
    }
}

#[derive(Deserialize)]
struct DctfResponse(HashMap<String, String>);

#[async_trait]
impl Submitter for DctfSubmitter {
    async fn submit(&self, flags: Vec<String>) -> Result<Vec<(String, FlagStatus)>, SubmitError> {
        if flags.is_empty() {
            return Ok(Vec::new());
        }

        // TODO max 200...

        let inst = Instant::now();

        let payload = json!({"flags": flags});
        let payload = serde_json::to_string(&payload)?;

        let request = self
            .client
            .post(&self.url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("Cookie", &self.cookie)
            .body(payload)
            .build()?;

        trace!("DCTF: Sending {:?}", request);

        let response = self.client.execute(request).await?;

        trace!("DCTF: Response {:?}", response);

        let response: DctfResponse = response.json().await?;

        let statuses: Vec<_> = response
            .0
            .into_iter()
            .map(|(flag, message)| {
                (
                    flag,
                    match message.as_str() {
                        "Flag is too old." => FlagStatus::Old,
                        "You cannot submit your own flag." => FlagStatus::Own,
                        "Invalid flag format." => FlagStatus::Invalid,
                        "Flag already submitted." => FlagStatus::Duplicate,
                        "Flag submitted." => FlagStatus::Ok,
                        other => {
                            warn!("Unknown dctf response '{}', assuming ERR", other);
                            FlagStatus::Error
                        }
                    },
                )
            })
            .collect();

        let elapsed = inst.elapsed();

        debug!(
            "Submitted {} flags in {}ms",
            flags.len(),
            elapsed.as_millis()
        );

        Ok(statuses)
    }
}
