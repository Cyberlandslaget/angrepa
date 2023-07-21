use diesel::prelude::*;

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = super::schema::exploits)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ExploitModel {
    pub id: String,
    pub running: bool,
    pub attack_target: Option<String>,
    pub docker_image: String,
    pub exploit_kind: String,
}

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = super::schema::flags)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct FlagModel {
    pub flag: String,
    pub tick: Option<i32>,
    pub stamp: Option<chrono::NaiveDateTime>,
    pub exploit_id: Option<String>,
    pub target_ip: Option<String>,
    pub flagstore: Option<String>,
}
