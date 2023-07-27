use crate::models::{ExecutionModel, ExploitModel, FlagModel, TargetModel};
use chrono::NaiveDateTime;
use color_eyre::Report;
use diesel::prelude::*;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use std::collections::HashMap;

use super::Db;

impl<'a> Db<'a> {
    pub fn exploits(&mut self) -> Result<Vec<ExploitModel>, Report> {
        use crate::schema::exploit::dsl::*;

        let exploits = exploit.load::<ExploitModel>(self.conn)?;

        Ok(exploits)
    }

    pub fn exploit(&mut self, exp_id: i32) -> Result<Vec<ExploitModel>, Report> {
        use crate::schema::exploit::dsl::*;

        let exploits = exploit
            .filter(id.eq(exp_id))
            .load::<ExploitModel>(self.conn)?;

        Ok(exploits)
    }

    pub fn exploit_flags_since(
        &mut self,
        exp_id: i32,
        since: NaiveDateTime,
    ) -> Result<Vec<FlagModel>, Report> {
        use crate::schema::*;

        let exploits = exploit::table
            .filter(exploit::id.eq(exp_id))
            .load::<ExploitModel>(self.conn)?;

        let flags = FlagModel::belonging_to(&exploits)
            .filter(flag::timestamp.ge(since))
            .load::<FlagModel>(self.conn)?;

        Ok(flags)
    }

    pub fn flags_since(&mut self, since: NaiveDateTime) -> Result<Vec<FlagModel>, Report> {
        use crate::schema::flag::dsl::*;

        let flags = flag
            .filter(timestamp.ge(since))
            .load::<FlagModel>(self.conn)?;

        Ok(flags)
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

    pub fn executions_since_extended(
        &mut self,
        since: NaiveDateTime,
    ) -> Result<Vec<(ExecutionModel, TargetModel, Vec<FlagModel>)>, Report> {
        use crate::schema::*;

        let executions = execution::table
            .inner_join(target::table.on(execution::target_id.eq(target::id)))
            .filter(execution::started_at.ge(since))
            .load::<(ExecutionModel, TargetModel)>(self.conn)?;

        let execution_ids = executions
            .iter()
            .map(|(e, _)| e.clone())
            .collect::<Vec<_>>();

        let relevant_flags: Vec<_> =
            FlagModel::belonging_to(&execution_ids).load::<FlagModel>(self.conn)?;

        let mut flags = HashMap::new();
        for flag in relevant_flags {
            flags
                .entry(flag.execution_id)
                .or_insert_with(Vec::new)
                .push(flag);
        }

        let mut output = vec![];
        for (exec, target) in executions {
            let flags = flags.get(&exec.id).unwrap_or(&vec![]).clone();
            output.push((exec, target, flags));
        }

        Ok(output)
    }

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
