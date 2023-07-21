use diesel::prelude::*;

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = super::schema::exploits)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Exploits {
    pub id: String,
    pub running: bool,
    pub attack_target: Option<String>,
    pub docker_image: String,
    pub exploit_kind: String,
}
