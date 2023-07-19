use super::{FlagStatus, SubmitError, Submitter};
use async_trait::async_trait;
use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::warn;

#[derive(Clone, Debug, Deserialize)]
pub struct FaustSubmitter {
    host: String,
    /// Reads until this is found
    header_suffix: String,
}

impl FaustSubmitter {
    pub fn new(host: String, header_suffix: String) -> Self {
        Self {
            host,
            header_suffix,
        }
    }
}

#[async_trait]
impl Submitter for FaustSubmitter {
    async fn submit(&self, flags: Vec<String>) -> Result<Vec<(String, FlagStatus)>, SubmitError> {
        let mut socket = tokio::net::TcpStream::connect(&self.host).await?;

        // send all flags
        let all_flags = flags.join("\n");
        socket.write_all(all_flags.as_bytes()).await?;

        // read all data
        let response = {
            let mut buf = [0u8; 1024];
            let mut read_text: Vec<u8> = Vec::new();
            loop {
                match socket.read(&mut buf).await {
                    Ok(n) => {
                        if n == 0 {
                            break;
                        }
                        read_text.extend(&buf[..n]);
                    }
                    Err(e) => {
                        return Result::Err(e.into());
                    }
                }
            }
            read_text
        };

        // extract responses
        let response = String::from_utf8_lossy(&response);
        let lines = {
            let (_preheader, body) = response
                .split_once(&self.header_suffix)
                .ok_or(SubmitError::FormatError)?;

            // remove any leading or trailing whitespace, just in case, they should never be part of
            // flagformat anyway
            let body = body.trim();
            let lines = body.split('\n').collect::<Vec<_>>();

            if lines.len() != flags.len() {
                return Err(SubmitError::FormatError);
            }

            lines
        };

        let mut statuses = Vec::new();
        for line in lines {
            // split twice on space to get 3 variables
            let (flag, rest) = line.split_once(' ').ok_or(SubmitError::FormatError)?;
            let (code, _msg) = rest.split_once(' ').ok_or(SubmitError::FormatError)?;

            let status = match code {
                "OK" => FlagStatus::Accepted,
                "DUP" => FlagStatus::Duplicate,
                "OWN" => FlagStatus::Own,
                "OLD" => FlagStatus::Old,
                "INV" => FlagStatus::Invalid,
                "ERR" => FlagStatus::Error,
                _ => {
                    warn!("Unknown flag status: {} for flag {}", code, flag);

                    FlagStatus::Unknown
                }
            };

            statuses.push((flag.to_string(), status));
        }

        Ok(statuses)
    }
}
