pub mod config;
pub mod db;
pub mod models;
pub mod schema;
pub mod wh;

use color_eyre::Report;
use diesel::{Connection, PgConnection};

pub fn db_connect(db_url: &String) -> Result<PgConnection, Report> {
    Ok(PgConnection::establish(db_url.as_str())?)
}
