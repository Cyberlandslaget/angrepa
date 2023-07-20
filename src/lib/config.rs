use argh::FromArgs;
use chrono::DateTime;
use color_eyre::{eyre::eyre, Report};
use serde::Deserialize;
use tracing::{debug, info};
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;

use super::wh::WebhookLayer;

#[derive(Debug, Deserialize)]
pub struct Common {
    /// round length
    pub tick: u64,
    pub format: String,
    pub start: DateTime<chrono::Utc>,
}

impl Common {
    pub async fn sleep_until_start(&self) {
        let current_time = chrono::Utc::now();
        let difference =
            std::cmp::max(self.start, current_time) - std::cmp::min(self.start, current_time);

        debug!("Start time: {:?}", self.start);
        debug!("Current time: {:?}", current_time);
        debug!("Difference: {:?}", difference);

        if current_time <= self.start {
            info!("Starts in {:?}. Sleeping...", difference.to_std().unwrap());
            tokio::time::sleep_until(tokio::time::Instant::now() + difference.to_std().unwrap())
                .await;
        }
    }

    pub fn current_tick(&self, current_time: DateTime<chrono::Utc>) -> i64 {
        let seconds_after_start = current_time - self.start;

        let ticks_after_start = seconds_after_start.num_seconds() / (self.tick as i64);

        ticks_after_start
    }
}

#[derive(Debug, Deserialize)]
pub struct Manager {
    pub http_listener: String,
    pub tcp_listener: String,
    pub submitter_name: String,
    pub submitter: toml::Table,
    pub fetcher_name: String,
    pub fetcher: toml::Table,
}

#[derive(Debug, Deserialize)]
pub struct Root {
    pub common: Common,
    pub manager: Manager,
    pub runner: toml::Value,
}

// common args, used by both manager and runner
#[derive(FromArgs)]
/// Angrapa
pub struct Args {
    /// path to toml configuration file
    #[argh(positional)]
    pub toml: String,

    /// enable debug logging
    #[argh(switch)]
    pub debug: bool,
}

impl Args {
    fn get_toml(&self) -> Result<toml::Value, Report> {
        let toml = std::fs::read_to_string(&self.toml)?;
        Ok(toml::from_str(&toml)?)
    }

    pub fn get_config(&self) -> Result<Root, Report> {
        let toml = std::fs::read_to_string(&self.toml)?;
        Ok(toml::from_str(&toml)?)
    }

    fn get_wh_url(&self) -> Result<Option<String>, Report> {
        // get the raw thing so that we dont panic on missing

        let url = {
            let toml = self.get_toml()?;
            let wh_url = toml
                .get("common")
                .ok_or(eyre!("missing common section"))?
                .get("webhook");

            if let Some(wh_url) = wh_url {
                let wh_url = wh_url
                    .as_str()
                    .ok_or(eyre!("webhook url is not a string"))?;

                wh_url.to_string()
            } else {
                return Ok(None);
            }
        };

        Ok(Some(url))
    }

    pub fn setup_logging(&self) -> Result<(), Report> {
        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(if self.debug {
                "debug,hyper=info"
            } else {
                "info"
            })
            .finish();

        let wh_url = self.get_wh_url()?;

        if let Some(wh_url) = wh_url {
            let url = wh_url.clone();
            let wh = WebhookLayer::new(wh_url);
            tracing::subscriber::set_global_default(subscriber.with(wh))?;

            info!("webhook url: {}", url);
        } else {
            tracing::subscriber::set_global_default(subscriber)?;

            info!("no webhook url");
        }

        Ok(())
    }
}
