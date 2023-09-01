use crate::models::{
    ExecutionInserter, ExecutionModel, ExploitInserter, ExploitModel, FlagInserter, FlagModel,
    TargetInserter, TargetModel,
};
use diesel::prelude::*;
use diesel::{ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl};
use lexical_sort::natural_lexical_cmp;

mod data;

pub struct Db<'a> {
    conn: &'a mut PgConnection,
}

#[derive(thiserror::Error, Debug)]
pub enum DbError {
    #[error("diesel error")]
    Diesel(#[from] diesel::result::Error),
}

impl<'a> Db<'a> {
    pub fn new(conn: &'a mut PgConnection) -> Self {
        Self { conn }
    }

    pub fn conn(&self) -> &PgConnection {
        self.conn
    }

    // exploits

    pub fn get_exploits(&mut self) -> Result<Vec<ExploitModel>, DbError> {
        use crate::schema::exploit::dsl::*;

        let exploits = exploit.load::<ExploitModel>(self.conn)?;

        Ok(exploits)
    }

    pub fn add_exploit(&mut self, exp: &ExploitInserter) -> Result<ExploitModel, DbError> {
        use crate::schema::exploit::dsl::*;

        Ok(diesel::insert_into(exploit)
            .values(exp)
            .get_result(self.conn)?)
    }

    pub fn start_exploit(&mut self, target_id: i32) -> Result<(), DbError> {
        use crate::schema::exploit::dsl::*;

        diesel::update(exploit.filter(id.eq(target_id)))
            .set(enabled.eq(true))
            .execute(self.conn)?;

        Ok(())
    }

    pub fn stop_exploit(&mut self, target_id: i32) -> Result<(), DbError> {
        use crate::schema::exploit::dsl::*;

        diesel::update(exploit.filter(id.eq(target_id)))
            .set(enabled.eq(false))
            .execute(self.conn)?;

        Ok(())
    }

    pub fn set_docker_containers(&mut self, ids: Vec<String>) -> Result<(), DbError> {
        use crate::schema::exploit::dsl::*;

        diesel::update(exploit)
            .set(docker_containers.eq(ids))
            .execute(self.conn)?;

        Ok(())
    }

    // execution

    pub fn add_execution(&mut self, exec: &ExecutionInserter) -> Result<ExecutionModel, DbError> {
        use crate::schema::execution::dsl::*;

        Ok(diesel::insert_into(execution)
            .values(exec)
            .get_result(self.conn)?)
    }

    // flag

    pub fn add_flag(&mut self, fl: &FlagInserter) -> Result<(), DbError> {
        use crate::schema::flag::dsl::*;

        diesel::insert_into(flag).values(fl).execute(self.conn)?;

        Ok(())
    }

    pub fn update_flag_status(
        &mut self,
        search_text: &str,
        new_status: &str,
    ) -> Result<(), DbError> {
        use crate::schema::flag::dsl::*;

        diesel::update(flag.filter(text.eq(search_text)))
            .set(status.eq(new_status))
            .execute(self.conn)?;

        Ok(())
    }

    pub fn get_unsubmitted_flags(&mut self) -> Result<Vec<FlagModel>, DbError> {
        use crate::schema::flag::dsl::*;

        let flags = flag
            .filter(submitted.eq(false))
            .load::<FlagModel>(self.conn)?;

        Ok(flags)
    }

    pub fn set_flag_submitted(&mut self, target_id: i32) -> Result<(), DbError> {
        use crate::schema::flag::dsl::*;

        diesel::update(flag.filter(id.eq(target_id)))
            .set(submitted.eq(true))
            .execute(self.conn)?;

        Ok(())
    }

    // service

    /// Ignores conflicts
    pub fn add_service_checked(&mut self, name_str: &str) -> Result<(), DbError> {
        use crate::schema::service::dsl::*;

        diesel::insert_into(service)
            .values(name.eq(name_str))
            .on_conflict(name)
            .do_nothing()
            .execute(self.conn)?;

        Ok(())
    }

    /// since service only has a name, only return a bool
    pub fn service_exists(&mut self, name_str: &str) -> Result<bool, DbError> {
        use crate::schema::service::dsl::*;

        // is there an entry with name = name_str?
        let exists = diesel::select(diesel::dsl::exists(service.filter(name.eq(name_str))))
            .get_result(self.conn)?;

        Ok(exists)
    }

    pub fn add_target(&mut self, trg: &TargetInserter) -> Result<(), DbError> {
        use crate::schema::target::dsl::*;

        diesel::insert_into(target).values(trg).execute(self.conn)?;

        Ok(())
    }

    pub fn get_latest_nop_target(&mut self, nop_ip: &str) -> Result<Option<TargetModel>, DbError> {
        use crate::schema::target;

        let out: Vec<_> = target::table
            .filter(target::team.eq(nop_ip))
            .order(target::created_at.asc())
            .limit(1)
            .load::<TargetModel>(self.conn)?;

        let out = out.get(0).cloned();

        Ok(out)
    }

    pub fn get_exploitable_targets_updating(
        &mut self,
        oldest: chrono::NaiveDateTime,
    ) -> Result<Vec<(Vec<TargetModel>, ExploitModel)>, DbError> {
        use crate::schema::{execution, exploit, target};

        // to be exploitable a target must
        // 1. not already be exploited by the specific exploit
        //       (but can be exploited by another exploit)
        // 2. have an active exploit pointing to it
        // 3. not be older than the N ticks/seconds where N is the max age of a flag
        //
        // targets will also be sorted by oldest first to prioritize flags that are about to expire

        let active_exploits = exploit::table
            .filter(exploit::enabled.eq(true))
            .load::<ExploitModel>(self.conn)?;

        let relevant_executions = ExecutionModel::belonging_to(&active_exploits)
            .filter(execution::finished_at.gt(oldest))
            .select(execution::target_id)
            .load::<i32>(self.conn)?;

        let mut target_exploits = Vec::new();

        for exploit in active_exploits {
            let mut targets: Vec<TargetModel> = target::table
                .filter(target::id.ne_all(&relevant_executions)) // 1.
                .filter(target::service.eq(&exploit.service)) // 2.
                .filter(target::created_at.gt(oldest)) // 3.
                .order(target::created_at.asc())
                .load::<TargetModel>(self.conn)?;

            // sort by ip to make viewing an adminer easier
            targets.sort_by(|a, b| natural_lexical_cmp(&a.team, &b.team));

            target_exploits.push((targets, exploit));
        }

        Ok(target_exploits)
    }

    /// Ignores conflicts
    pub fn add_team_checked(&mut self, ip_str: &str) -> Result<(), DbError> {
        use crate::schema::team::dsl::*;

        diesel::insert_into(team)
            .values(ip.eq(ip_str))
            .on_conflict(ip)
            .do_nothing()
            .execute(self.conn)?;

        Ok(())
    }
}
