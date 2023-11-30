#[derive(Debug, Clone)]
pub struct ExploitInserter {
    pub name: String,
    pub service: String,
    pub blacklist: Vec<String>,
    pub enabled: bool,
    pub docker_image: String,
    pub docker_containers: Vec<String>,
    pub pool_size: i32,
}

#[derive(Debug, Clone)]
pub struct ExecutionInserter {
    pub exploit_id: i32,
    pub output: String,
    pub exit_code: i32,
    pub started_at: chrono::NaiveDateTime,
    pub finished_at: chrono::NaiveDateTime,
    pub target_id: i32,
}

#[derive(Debug, Clone)]
pub struct FlagInserter {
    pub text: String,
    pub status: String,
    pub submitted: bool,
    pub timestamp: chrono::NaiveDateTime,
    pub execution_id: i32,
    pub exploit_id: i32,
}

#[derive(Debug, Clone)]
pub struct TargetInserter {
    pub flag_id: String,
    pub service: String,
    /// FOREIGN KEY
    pub team: String,
    /// FOREIGN KEY
    pub created_at: chrono::NaiveDateTime,
    pub target_tick: i32,
}
