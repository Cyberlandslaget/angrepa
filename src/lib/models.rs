use diesel::prelude::*;

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = super::schema::exploit)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ExploitModel {
    pub id: i32,
    pub name: String,
    pub service: String,
    pub blacklist: String,
    pub docker_image: String,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = super::schema::exploit)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ExploitInserter {
    pub name: String,
    pub service: String,
    pub blacklist: String,
    pub docker_image: String,
}

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = super::schema::execution)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ExecutionModel {
    pub id: i32,
    pub exploit_id: i32,
    pub output: String,
    pub started_at: chrono::NaiveDateTime,
    pub finished_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = super::schema::flag)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct FlagModel {
    pub id: i32,
    pub text: String,
    pub status: String,
    pub timestamp: chrono::NaiveDateTime,
    pub execution_id: i32,
    pub exploit_id: i32,
}
