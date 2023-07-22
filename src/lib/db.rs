use color_eyre::Report;
use diesel::{PgConnection, RunQueryDsl};

use crate::models::{ExecutionInserter, ExploitInserter, ExploitModel};

pub struct Db {
    conn: PgConnection,
}

impl Db {
    pub fn new(conn: PgConnection) -> Self {
        Self { conn }
    }

    pub fn conn(&self) -> &PgConnection {
        &self.conn
    }

    // exploits

    pub fn get_exploits(&mut self) -> Result<Vec<ExploitModel>, Report> {
        use crate::schema::exploit::dsl::*;

        let exploits = exploit.load::<ExploitModel>(&mut self.conn)?;

        Ok(exploits)
    }

    pub fn add_exploit(&mut self, exp: &ExploitInserter) -> Result<(), Report> {
        use crate::schema::exploit::dsl::*;

        diesel::insert_into(exploit)
            .values(exp)
            .execute(&mut self.conn)?;

        Ok(())
    }

    // execution

    pub fn add_execution(&mut self, exec: &ExecutionInserter) -> Result<(), Report> {
        use crate::schema::execution::dsl::*;

        diesel::insert_into(execution)
            .values(exec)
            .execute(&mut self.conn)?;

        Ok(())
    }
}
