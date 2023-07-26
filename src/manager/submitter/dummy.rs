use async_trait::async_trait;
use rand::Rng;

use super::{FlagStatus, SubmitError, Submitter};

#[derive(Clone, Debug)]
pub struct DummySubmitter {}

#[async_trait]
impl Submitter for DummySubmitter {
    async fn submit(&self, flags: Vec<String>) -> Result<Vec<(String, FlagStatus)>, SubmitError> {
        let statuses = flags
            .into_iter()
            .map(|flag| {
                let mut rng = rand::thread_rng();
                let r = rng.gen_range(0..=99);
                match r {
                    0..=49 => (flag, FlagStatus::Ok),
                    50..=59 => (flag, FlagStatus::Duplicate),
                    60..=69 => (flag, FlagStatus::Own),
                    70..=79 => (flag, FlagStatus::Old),
                    80..=89 => (flag, FlagStatus::Invalid),
                    90..=99 => (flag, FlagStatus::Error),
                    _ => unreachable!(),
                }
            })
            .collect();
        Ok(statuses)
    }
}
