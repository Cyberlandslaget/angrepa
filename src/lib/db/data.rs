use color_eyre::Report;
use diesel::RunQueryDsl;

use crate::models::ExploitModel;

use super::Db;

impl<'a> Db<'a> {
    pub fn exploits_all(&mut self) -> Result<Vec<ExploitModel>, Report> {
        use crate::schema::exploit::dsl::*;

        let exploits = exploit.load::<ExploitModel>(self.conn)?;

        Ok(exploits)
    }
}
