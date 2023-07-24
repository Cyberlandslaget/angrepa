use crate::models::{ExploitModel, FlagModel};
use color_eyre::Report;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};

use super::Db;

impl<'a> Db<'a> {
    pub fn exploits_all(&mut self) -> Result<Vec<ExploitModel>, Report> {
        use crate::schema::exploit::dsl::*;

        let exploits = exploit.load::<ExploitModel>(self.conn)?;

        Ok(exploits)
    }

    pub fn exploits_one(&mut self, exp_id: i32) -> Result<Vec<ExploitModel>, Report> {
        use crate::schema::exploit::dsl::*;

        let exploits = exploit
            .filter(id.eq(exp_id))
            .load::<ExploitModel>(self.conn)?;

        Ok(exploits)
    }

    pub fn flags_all(&mut self) -> Result<Vec<FlagModel>, Report> {
        use crate::schema::flag::dsl::*;

        let flags = flag.load::<FlagModel>(self.conn)?;

        Ok(flags)
    }
}
