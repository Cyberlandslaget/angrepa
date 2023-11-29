use crate::models::{ExecutionModel, ExploitModel, FlagModel};
use chrono::NaiveDateTime;
use color_eyre::Report;
use diesel::prelude::*;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};

use super::Db;

impl<'a> Db<'a> {
    pub fn service_exploits(&mut self, service_name: &String) -> Result<Vec<ExploitModel>, Report> {
        use crate::schema::exploit::dsl::*;

        let exploits = exploit
            .filter(service.eq(service_name))
            .load::<ExploitModel>(self.conn)?;

        Ok(exploits)
    }

    pub fn service_flags_since(
        &mut self,
        service_name: &String,
        since: NaiveDateTime,
    ) -> Result<Vec<FlagModel>, Report> {
        use crate::schema::*;

        let exploits = exploit::table
            .filter(exploit::service.eq(service_name))
            .load::<ExploitModel>(self.conn)?;

        let flags = FlagModel::belonging_to(&exploits)
            .filter(flag::timestamp.ge(since))
            .load::<FlagModel>(self.conn)?;

        Ok(flags)
    }

    pub fn service_executions_since(
        &mut self,
        service_name: &String,
        since: NaiveDateTime,
    ) -> Result<Vec<ExecutionModel>, Report> {
        use crate::schema::*;

        let exploits = exploit::table
            .filter(exploit::service.eq(service_name))
            .load::<ExploitModel>(self.conn)?;

        let executions = ExecutionModel::belonging_to(&exploits)
            .filter(execution::started_at.ge(since))
            .load::<ExecutionModel>(self.conn)?;

        Ok(executions)
    }
}
