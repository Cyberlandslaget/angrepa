use diesel::prelude::*;
use serde::Serialize;

#[derive(Serialize, Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = super::schema::exploit)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ExploitModel {
    pub id: i32,
    pub name: String,
    pub service: String,
    pub blacklist: Vec<Option<String>>,
    pub enabled: bool,
    pub docker_image: String,
    pub docker_containers: Vec<Option<String>>,
    pub pool_size: i32,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = super::schema::exploit)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ExploitInserter {
    pub name: String,
    pub service: String,
    pub blacklist: Vec<String>,
    pub enabled: bool,
    pub docker_image: String,
    pub docker_containers: Vec<String>,
    pub pool_size: i32,
}

#[derive(Serialize, Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = super::schema::team)]
pub struct TeamModel {
    pub ip: String,
    pub name: Option<String>,
}

#[derive(Serialize, Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = super::schema::service)]
pub struct ServiceModel {
    pub name: String,
}

#[derive(Serialize, Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = super::schema::flagid)]
#[diesel(belongs_to(ServiceModel, foreign_key = service))]
#[diesel(belongs_to(team, foreign_key = team))]
pub struct FlagidModel {
    pub id: i32,
    pub flag_id: String,
    pub service: String,
    pub team: String,
}

#[derive(
    Serialize, Debug, Clone, Queryable, Selectable, Insertable, Associations, Identifiable,
)]
#[diesel(table_name = super::schema::execution)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(belongs_to(ExploitModel, foreign_key = exploit_id))]
#[diesel(belongs_to(FlagidModel, foreign_key = flagid_id))]
pub struct ExecutionModel {
    pub id: i32,
    pub exploit_id: i32,
    pub output: String,
    pub started_at: chrono::NaiveDateTime,
    pub finished_at: chrono::NaiveDateTime,
    pub flagid_id: i32,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = super::schema::execution)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ExecutionInserter {
    pub exploit_id: i32,
    pub output: String,
    pub started_at: chrono::NaiveDateTime,
    pub finished_at: chrono::NaiveDateTime,
}

#[derive(
    Serialize, Debug, Clone, Queryable, Selectable, Insertable, Identifiable, Associations,
)]
#[diesel(table_name = super::schema::flag)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(belongs_to(ExecutionModel, foreign_key = execution_id))]
#[diesel(belongs_to(ExploitModel, foreign_key = exploit_id))]
pub struct FlagModel {
    pub id: i32,
    pub text: String,
    pub status: String,
    pub submitted: bool,
    pub timestamp: chrono::NaiveDateTime,
    pub execution_id: i32,
    pub exploit_id: i32,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = super::schema::flag)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct FlagInserter {
    pub text: String,
    pub status: String,
    pub submitted: bool,
    pub timestamp: chrono::NaiveDateTime,
    pub execution_id: i32,
    pub exploit_id: i32,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = super::schema::flagid)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct FlagidInserter {
    pub flag_id: String,
    /// FOREIGN KEY
    pub service: String,
    /// FOREIGN KEY
    pub team: String,
}
