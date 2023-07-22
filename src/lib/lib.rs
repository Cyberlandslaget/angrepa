pub mod config;
pub mod models;
pub mod schema;
pub mod wh;

use color_eyre::Report;
use diesel::{Connection, PgConnection};
use dotenvy::dotenv;
use std::env;

pub fn db_connect() -> Result<PgConnection, Report> {
    dotenv()?;

    let url = env::var("DATABASE_URL")?;

    Ok(PgConnection::establish(&url)?)
}
