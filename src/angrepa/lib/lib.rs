pub mod config;
pub mod db;
pub mod models;
pub mod schema;
pub mod wh;

use color_eyre::Report;
use diesel::r2d2::ConnectionManager;
use diesel::r2d2::Pool;
use diesel::{Connection, PgConnection};

pub fn db_connect(url: &str) -> Result<PgConnection, Report> {
    Ok(PgConnection::establish(url)?)
}

pub fn get_connection_pool(url: &String) -> Result<Pool<ConnectionManager<PgConnection>>, Report> {
    let manager = ConnectionManager::<PgConnection>::new(url);
    Ok(Pool::builder().test_on_check_out(true).build(manager)?)
}
