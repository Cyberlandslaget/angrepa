use argh::FromArgs;
use chrono::DateTime;
use color_eyre::{eyre::eyre, Report};
use serde::Deserialize;
use tokio::time::{interval_at, MissedTickBehavior};
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

    /// Returns a interval with a duration of tick seconds
    pub async fn get_tick_interval(
        &self,
        offset: tokio::time::Duration,
    ) -> Result<tokio::time::Interval, Report> {
        let time_since_start = chrono::Utc::now() - self.start;
        let start = tokio::time::Instant::now() - time_since_start.to_std()?;
        let tick = tokio::time::Duration::from_secs(self.tick);

        // offset by e.g. 1s to be safe we don't go too early
        let start = start + offset;

        let mut interval = interval_at(start, tick);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        Ok(interval)
    }

    // see the test for exactly how it works
    pub fn current_tick(&self, current_time: DateTime<chrono::Utc>) -> i64 {
        let seconds_after_start = current_time - self.start;

        // ew float
        let ticks_after_start = (seconds_after_start.num_seconds() as f64) / (self.tick as f64);
        // round down, so ex. 1ms before start, we're at -1, not 0
        let ticks_after_start = ticks_after_start.floor() as i64;

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
pub struct Runner {
    pub http_server: String,
}

#[derive(Debug, Deserialize)]
pub struct Root {
    pub common: Common,
    pub manager: Manager,
    pub runner: Runner,
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

#[cfg(test)]
mod tests {
    use super::Common;

    #[test]
    fn tick_rounding() {
        // CTF starts at 2020-01-01 05:00
        let common = Common {
            tick: 60,
            format: "".to_string(),
            start: chrono::DateTime::from_utc(
                chrono::NaiveDateTime::new(
                    chrono::NaiveDate::from_ymd(2020, 1, 1),
                    chrono::NaiveTime::from_hms(5, 0, 0),
                ),
                chrono::Utc,
            ),
        };

        // exactly at start
        let zero = common.current_tick(common.start);
        assert_eq!(zero, 0);

        // right before start
        let right_before_start = common.current_tick(common.start - chrono::Duration::seconds(1));
        assert_eq!(right_before_start, -1);

        // right after start
        let right_after_start = common.current_tick(common.start + chrono::Duration::seconds(1));
        assert_eq!(right_after_start, 0);

        // exactly one hour after start
        let one_hour_after_start = common.current_tick(common.start + chrono::Duration::hours(1));
        assert_eq!(one_hour_after_start, 60);

        // 59 minutes, 59 seconds after start
        let almost_one_hour_after_start = common.current_tick(
            common.start + chrono::Duration::minutes(59) + chrono::Duration::seconds(59),
        );
        assert_eq!(almost_one_hour_after_start, 59);

        // one hour before start
        let one_hour_before_start = common.current_tick(common.start - chrono::Duration::hours(1));
        assert_eq!(one_hour_before_start, -60);
    }
}
