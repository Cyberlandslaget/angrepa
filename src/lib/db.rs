use color_eyre::Report;
use diesel::{ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl};

use crate::models::{
    ExecutionInserter, ExecutionModel, ExploitInserter, ExploitModel, FlagInserter, FlagModel,
    TargetInserter, TargetModel,
};

pub struct Db<'a> {
    conn: &'a mut PgConnection,
}

mod data;

impl<'a> Db<'a> {
    pub fn new(conn: &'a mut PgConnection) -> Self {
        Self { conn }
    }

    pub fn conn(&self) -> &PgConnection {
        self.conn
    }

    // exploits

    pub fn get_exploits(&mut self) -> Result<Vec<ExploitModel>, Report> {
        use crate::schema::exploit::dsl::*;

        let exploits = exploit.load::<ExploitModel>(self.conn)?;

        Ok(exploits)
    }

    pub fn add_exploit(&mut self, exp: &ExploitInserter) -> Result<ExploitModel, Report> {
        use crate::schema::exploit::dsl::*;

        Ok(diesel::insert_into(exploit)
            .values(exp)
            .get_result(self.conn)?)
    }

    pub fn start_exploit(&mut self, target_id: i32) -> Result<(), Report> {
        use crate::schema::exploit::dsl::*;

        diesel::update(exploit.filter(id.eq(target_id)))
            .set(enabled.eq(true))
            .execute(self.conn)?;

        Ok(())
    }

    pub fn stop_exploit(&mut self, target_id: i32) -> Result<(), Report> {
        use crate::schema::exploit::dsl::*;

        diesel::update(exploit.filter(id.eq(target_id)))
            .set(enabled.eq(false))
            .execute(self.conn)?;

        Ok(())
    }

    pub fn set_docker_containers(&mut self, ids: Vec<String>) -> Result<(), Report> {
        use crate::schema::exploit::dsl::*;

        diesel::update(exploit)
            .set(docker_containers.eq(ids))
            .execute(self.conn)?;

        Ok(())
    }

    // execution

    pub fn add_execution(&mut self, exec: &ExecutionInserter) -> Result<ExecutionModel, Report> {
        use crate::schema::execution::dsl::*;

        Ok(diesel::insert_into(execution)
            .values(exec)
            .get_result(self.conn)?)
    }

    // flag

    pub fn add_flag(&mut self, fl: &FlagInserter) -> Result<(), Report> {
        use crate::schema::flag::dsl::*;

        diesel::insert_into(flag).values(fl).execute(self.conn)?;

        Ok(())
    }

    pub fn update_flag_status(
        &mut self,
        search_text: &str,
        new_status: &str,
    ) -> Result<(), Report> {
        use crate::schema::flag::dsl::*;

        diesel::update(flag.filter(text.eq(search_text)))
            .set(status.eq(new_status))
            .execute(self.conn)?;

        Ok(())
    }

    pub fn get_unsubmitted_flags(&mut self) -> Result<Vec<FlagModel>, Report> {
        use crate::schema::flag::dsl::*;

        let flags = flag
            .filter(submitted.eq(false))
            .load::<FlagModel>(self.conn)?;

        Ok(flags)
    }

    pub fn set_flag_submitted(&mut self, target_id: i32) -> Result<(), Report> {
        use crate::schema::flag::dsl::*;

        diesel::update(flag.filter(id.eq(target_id)))
            .set(submitted.eq(true))
            .execute(self.conn)?;

        Ok(())
    }

    // service

    pub fn add_service(&mut self, name_str: &str) -> Result<(), Report> {
        use crate::schema::service::dsl::*;

        diesel::insert_into(service)
            .values(name.eq(name_str))
            .execute(self.conn)?;

        Ok(())
    }

    /// since service only has a name, only return a bool
    pub fn service_exists(&mut self, name_str: &str) -> Result<bool, Report> {
        use crate::schema::service::dsl::*;

        // is there an entry with name = name_str?
        let exists = diesel::select(diesel::dsl::exists(service.filter(name.eq(name_str))))
            .get_result(self.conn)?;

        Ok(exists)
    }

    pub fn add_target(&mut self, trg: &TargetInserter) -> Result<(), Report> {
        use crate::schema::target::dsl::*;

        diesel::insert_into(target).values(trg).execute(self.conn)?;

        Ok(())
    }

    pub fn get_exploitable_target(
        &mut self,
        oldest: chrono::NaiveDateTime,
    ) -> Result<Vec<(Vec<TargetModel>, ExploitModel)>, Report> {
        use crate::schema::{exploit, target};

        // to be exploitable a target must
        // 1. not be exploited (exploited = false)
        // 2. have an active exploit pointing to it
        // 3. not be older than the N ticks where N is the number of old ticks you can exploit
        //
        // targets will also be sorted by oldest first to prioritize flags that are about to expire

        let active_exploits = exploit::table
            .filter(exploit::enabled.eq(true))
            .load::<ExploitModel>(self.conn)?;

        let target_exploits = active_exploits
            .into_iter()
            .map(|exploit| {
                let target = target::table
                    .filter(target::exploited.eq(false)) // 1.
                    .filter(target::service.eq(&exploit.service)) // 2.
                    .filter(target::created_at.gt(oldest)) // 3.
                    .order(target::created_at.asc())
                    .load::<TargetModel>(self.conn)
                    .unwrap();

                (target, exploit)
            })
            .collect::<Vec<(Vec<TargetModel>, ExploitModel)>>();

        Ok(target_exploits)
    }

    pub fn target_exploited(&mut self, target_id: i32) -> Result<(), Report> {
        use crate::schema::target::dsl::*;

        diesel::update(target.filter(id.eq(target_id)))
            .set(exploited.eq(true))
            .execute(self.conn)?;

        Ok(())
    }

    pub fn add_team(&mut self, ip_str: &str) -> Result<(), Report> {
        use crate::schema::team::dsl::*;

        diesel::insert_into(team)
            .values(ip.eq(ip_str))
            .execute(self.conn)?;

        Ok(())
    }
}
