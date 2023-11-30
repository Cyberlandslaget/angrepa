#[derive(sqlx::FromRow, serde::Serialize, Debug, Clone)]
pub struct Team {
    pub ip: String,
    pub name: Option<String>,
}

#[derive(sqlx::FromRow, serde::Serialize, Debug, Clone)]
pub struct Service {
    pub name: String,
}

#[derive(sqlx::FromRow, sqlx::Type, serde::Serialize, Debug, Clone)]
pub struct Target {
    pub id: i32,
    pub flag_id: String,
    /// FOREIGN KEY
    pub service: String,
    pub team: String,
    /// FOREIGN KEY
    pub created_at: chrono::NaiveDateTime,
    pub target_tick: i32,
}

pub struct TargetInserter {
    // no flagid
    pub flag_id: String,
    /// FOREIGN KEY
    pub service: String,
    pub team: String,
    /// FOREIGN KEY
    pub created_at: chrono::NaiveDateTime,
    pub target_tick: i32,
}

#[derive(sqlx::FromRow, serde::Serialize, Debug, Clone, serde::Deserialize)]
pub struct Exploit {
    pub id: i32,
    pub name: String,
    // FOREIGN KEY
    pub service: String,
    pub blacklist: Vec<String>,
    pub enabled: bool,
    pub docker_image: String,
    pub docker_containers: Vec<String>,
    pub pool_size: i32,
}

#[derive(sqlx::FromRow, sqlx::Type, serde::Serialize, Debug, Clone)]
pub struct Flag {
    pub id: i32,
    pub text: String,
    pub status: String,
    pub submitted: bool,
    pub timestamp: chrono::NaiveDateTime,
    // FOREIGN KEY
    pub execution_id: i32,
    // FOREIGN KEY
    pub exploit_id: i32,
}

#[derive(sqlx::FromRow, sqlx::Type, serde::Serialize, Debug, Clone)]
pub struct Execution {
    pub id: i32,
    // FOREIGN KEY
    pub exploit_id: i32,
    pub output: String,
    pub exit_code: i32,
    pub started_at: chrono::NaiveDateTime,
    pub finished_at: chrono::NaiveDateTime,
    // FOREIGN KEY
    pub target_id: i32,
}
