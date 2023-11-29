#![allow(clippy::needless_raw_string_hashes)]
pub mod config;
pub mod data_types;
pub mod db;
pub mod models;
pub mod schema;
pub mod types;
pub mod wh;

use color_eyre::Report;
use diesel::r2d2::ConnectionManager;
use diesel::r2d2::Pool;
use diesel::{Connection, PgConnection};
use std::time::Duration;

// TODO test a reasonable minimum (30 is default and way too high)
const GET_TIMEOUT: Duration = Duration::from_secs(5);

pub fn db_connect(url: &str) -> Result<PgConnection, Report> {
    Ok(PgConnection::establish(url)?)
}

pub fn get_connection_pool(url: &str) -> Result<Pool<ConnectionManager<PgConnection>>, Report> {
    let manager = ConnectionManager::<PgConnection>::new(url);
    Ok(Pool::builder()
        .max_size(100)
        .test_on_check_out(true)
        .connection_timeout(GET_TIMEOUT)
        .build(manager)?)
}
