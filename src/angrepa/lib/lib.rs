#![allow(clippy::needless_raw_string_hashes)]
pub mod config;
pub mod data_types;
pub mod db;
pub mod inserter;
pub mod types;
pub mod wh;

use color_eyre::Report;
use db::Db;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

// TODO test a reasonable minimum (30 is default and way too high)
// however, we also dont want things to fail, so idk.
const GET_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn db_connect(url: &str) -> Result<Db, Report> {
    Ok(Db::wrap(
        PgPoolOptions::new()
            .acquire_timeout(GET_TIMEOUT)
            .connect(url)
            .await?,
    ))
}
