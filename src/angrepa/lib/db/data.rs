use crate::models::{
    ExecutionModel, ExploitModel, FlagModel, ServiceModel, TargetModel, TeamModel,
};
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

    pub fn exploit(&mut self, exp_id: i32) -> Result<ExploitModel, Report> {
        use crate::schema::exploit::dsl::*;

        let expl = exploit
            .filter(id.eq(exp_id))
            .first::<ExploitModel>(self.conn)?;

        Ok(expl)
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

    pub fn exploit_edit_config(
        &mut self,
        exp_id: i32,
        exp_name: String,
        exp_blacklist: Vec<String>,
        exp_pool_size: i32,
    ) -> Result<(), Report> {
        use crate::schema::exploit::dsl::*;

        diesel::update(exploit.filter(id.eq(exp_id)))
            .set((
                name.eq(exp_name),
                blacklist.eq(exp_blacklist),
                pool_size.eq(exp_pool_size),
            ))
            .execute(self.conn)?;

        Ok(())
    }

    pub fn teams(&mut self) -> Result<Vec<TeamModel>, Report> {
        use crate::schema::team::dsl::*;

        let teams = team.load::<TeamModel>(self.conn)?;

        Ok(teams)
    }

    pub fn team_by_ip(&mut self, t_ip: String) -> Result<TeamModel, Report> {
        use crate::schema::team::dsl::*;

        Ok(team.filter(ip.eq(t_ip)).first::<TeamModel>(self.conn)?)
    }

    pub fn team_set_name(&mut self, team_ip: String, team_name: String) -> Result<(), Report> {
        use crate::schema::team::dsl::*;

        diesel::update(team.filter(ip.eq(team_ip)))
            .set(name.eq(team_name))
            .execute(self.conn)?;

        Ok(())
    }

    pub fn services(&mut self) -> Result<Vec<ServiceModel>, Report> {
        use crate::schema::service::dsl::*;

        let services = service.load::<ServiceModel>(self.conn)?;

        Ok(services)
    }

    pub fn flags_since(&mut self, since: NaiveDateTime) -> Result<Vec<FlagModel>, Report> {
        use crate::schema::flag::dsl::*;

        let flags = flag
            .filter(timestamp.ge(since))
            .load::<FlagModel>(self.conn)?;

        Ok(flags)
    }

    pub fn flags_since_extended(
        &mut self,
        since: NaiveDateTime,
    ) -> Result<Vec<(FlagModel, ExecutionModel, TargetModel)>, Report> {
        use crate::schema::*;

        let flags = flag::table
            .inner_join(execution::table.on(flag::execution_id.eq(execution::id)))
            .inner_join(target::table.on(execution::target_id.eq(target::id)))
            .filter(flag::timestamp.ge(since))
            .load::<(FlagModel, ExecutionModel, TargetModel)>(self.conn)?;

        Ok(flags)
    }

    pub fn flags_by_id_extended(
        &mut self,
        ids: Vec<i32>,
    ) -> Result<Vec<(FlagModel, ExecutionModel, TargetModel)>, Report> {
        use crate::schema::*;

        let flags = flag::table
            .inner_join(execution::table.on(flag::execution_id.eq(execution::id)))
            .inner_join(target::table.on(execution::target_id.eq(target::id)))
            .filter(flag::id.eq_any(ids))
            .load::<(FlagModel, ExecutionModel, TargetModel)>(self.conn)?;

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

    pub fn executions_by_id_extended(
        &mut self,
        ids: Vec<i32>,
    ) -> Result<Vec<(ExecutionModel, TargetModel, Vec<FlagModel>)>, Report> {
        use crate::schema::*;

        let executions = execution::table
            .filter(execution::id.eq_any(ids))
            .inner_join(target::table.on(execution::target_id.eq(target::id)))
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
