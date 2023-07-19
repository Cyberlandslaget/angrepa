use async_trait::async_trait;

use super::{FlagStatus, SubmitError, Submitter};

#[derive(Clone, Debug)]
pub struct DummySubmitter {}

#[async_trait]
impl Submitter for DummySubmitter {
    async fn submit(&self, flags: Vec<String>) -> Result<Vec<(String, FlagStatus)>, SubmitError> {
        let statuses = flags
            .into_iter()
            .map(|flag| (flag, FlagStatus::Accepted))
            .collect();
        Ok(statuses)
    }
}
