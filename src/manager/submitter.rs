use angrapa::config;
use async_trait::async_trait;
use color_eyre::{eyre::eyre, Report};
use thiserror::Error;

// implementations
mod faust;
pub use faust::FaustSubmitter;
mod dummy;
pub use dummy::DummySubmitter;

#[derive(Debug)]
pub enum Submitters {
    Dummy(DummySubmitter),
    Faust(FaustSubmitter),
}

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
#[derive(Debug, Clone, Copy)]
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

impl Submitters {
    pub fn from_conf(manager: &config::Manager) -> Result<Self, Report> {
        match manager.submitter_name.as_str() {
            "dummy" => Ok(Self::Dummy(DummySubmitter {})),
            "faust" => {
                let host = manager
                    .submitter
                    .get("host")
                    .ok_or(eyre!("Faust submitter requires host"))?;

                let host = match host {
                    toml::Value::String(s) => s.clone(),
                    _ => return Err(eyre!("Faust submitter host must be a string")),
                };

                let header_suffix = manager
                    .submitter
                    .get("header_suffix")
                    .ok_or(eyre!("Faust submitter requires header_suffix"))?;

                let header_suffix = match header_suffix {
                    toml::Value::String(s) => s.clone(),
                    _ => return Err(eyre!("Faust submitter header_suffix must be a string")),
                };

                let faust = FaustSubmitter::new(host, header_suffix);

                Ok(Self::Faust(faust))
            }
            _ => Err(eyre!("Unknown submitter name {}", manager.submitter_name)),
        }
    }
}
