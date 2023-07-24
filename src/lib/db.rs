use color_eyre::Report;
use diesel::{ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl};

use crate::models::{
    ExecutionInserter, ExecutionModel, ExploitInserter, ExploitModel, FlagInserter, FlagModel,
};

pub struct Db<'a> {
    conn: &'a mut PgConnection,
}

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
}
