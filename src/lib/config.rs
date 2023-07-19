use argh::FromArgs;
use serde::Deserialize;
#[derive(Debug, Deserialize)]
pub struct Common {
    pub tick: u64,
    pub format: String,
    pub start: chrono::DateTime<chrono::Utc>,
}

impl Common {
    pub async fn sleep_until_start(&self) {
        let current_time = chrono::Utc::now();
        let difference =
            std::cmp::max(self.start, current_time) - std::cmp::min(self.start, current_time);

        if current_time <= self.start {
            println!("Starts in {:?}. Sleeping...", difference.to_std().unwrap());
            tokio::time::sleep_until(tokio::time::Instant::now() + difference.to_std().unwrap())
                .await;
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Manager {
    pub http_listener: String,
    pub tcp_listener: String,
    pub submitter_name: String,
    pub submitter: toml::Table,
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
}
