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
    pub sent: bool,
    pub status: Option<String>,
}

impl Default for FlagModel {
    fn default() -> Self {
        Self {
            flag: "".to_string(),
            tick: None,
            stamp: None,
            exploit_id: None,
            target_ip: None,
            flagstore: None,
            sent: false,
            status: None,
        }
    }
}

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = super::schema::runlogs)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct RunlogModel {
    pub id: i32,
    pub from_exploit_id: String,
    pub from_ip: String,
    pub tick: i32,
    pub stamp: chrono::NaiveDateTime,
    pub content: String,
}
