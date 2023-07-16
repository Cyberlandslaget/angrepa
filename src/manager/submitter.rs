use async_trait::async_trait;
use thiserror::Error;

// implementations
mod ecsc;

/// Did not manage to submit
#[derive(Error, Debug)]
pub enum SubmitError {
    #[error("Network error")]
    NetworkError(#[from] std::io::Error),
    #[error("Format error")]
    /// The format of the response was not as expected
    FormatError,
}

/// Adapted from https://web.archive.org/web/20230325144340/https://docs.ecsc2022.eu/ad_platform/
pub enum FlagStatus {
    Accepted,
    Duplicate,
    Own,
    Old,
    Invalid,
    /// Server refused flag
    Error,
    /// Didn't understand the response
    Unknown,
}

/// Implements the low-level operation of submitting a bunch of flags
#[async_trait]
pub trait Submitter {
    async fn submit(&self, flags: Vec<String>) -> Result<Vec<(String, FlagStatus)>, SubmitError>;
}
