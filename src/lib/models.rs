use diesel::prelude::*;

#[derive(Queryable, Selectable, Insertable, Debug)]
#[diesel(table_name = super::schema::exploits)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ExploitModel {
    pub id: String,
    pub running: bool,
    pub attack_target: Option<String>,
    pub docker_image: String,
    pub exploit_kind: String,
}
