use chrono::NaiveDateTime;
use color_eyre::Report;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};

use crate::models::{ExecutionModel, ExploitModel, FlagModel};

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

    pub fn flags_since(&mut self, since: NaiveDateTime) -> Result<Vec<FlagModel>, Report> {
        use crate::schema::flag::dsl::*;

        let flags = flag
            .filter(timestamp.ge(since))
            .load::<FlagModel>(self.conn)?;

        Ok(flags)
    }

    pub fn executions_all(&mut self) -> Result<Vec<ExecutionModel>, Report> {
        use crate::schema::execution::dsl::*;

        let executions = execution.load::<ExecutionModel>(self.conn)?;

        Ok(executions)
    }

    pub fn executions_since(
        &mut self,
        since: NaiveDateTime,
    ) -> Result<Vec<ExecutionModel>, Report> {
        use crate::schema::execution::dsl::*;

        let executions = execution
            .filter(started_at.ge(since))
            .load::<ExecutionModel>(self.conn)?;

        Ok(executions)
    }
}
