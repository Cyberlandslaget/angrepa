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

// TODO test a reasonable minimum (30s is default and way too high)
// however, we also dont want things to fail, so idk.

// on my laptop
// - 150ms immediately crashes
// - 300ms seemingly works well, but sometimes errors on new connections
//
// takeaways:
// - only spawn pools on startup, then keep those, possibly clone them
// - 10s should be way more than enough, however we should test with lower values
const GET_TIMEOUT: Duration = Duration::from_millis(10_000);

// no clue
const MAX_CONS: u32 = 50;

pub async fn db_connect(url: &str) -> Result<Db, Report> {
    Ok(Db::wrap(
        PgPoolOptions::new()
            .max_connections(MAX_CONS)
            .acquire_timeout(GET_TIMEOUT)
            .connect(url)
            .await?,
    ))
}
