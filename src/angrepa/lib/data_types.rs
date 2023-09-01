use crate::models::{ExecutionModel, ExploitModel, FlagModel, TargetModel};
use chrono::NaiveDateTime;
use tabled::Tabled;

#[derive(serde::Serialize, PartialEq, Clone)]
// jhonnny boy provided this
pub struct ExecutionData {
    pub exit_code: i32, // whatver no point in panicing here cus its not u8
    pub exploit_id: i32,
    pub finished_at: NaiveDateTime,
    pub id: i32,
    pub output: String,
    pub started_at: NaiveDateTime,
    pub target_id: i32,
    pub service: String,
    pub target_tick: i32,
    pub team: String,
}

impl ExecutionData {
    pub fn from_models(exec: ExecutionModel, target: TargetModel) -> Self {
        Self {
            exit_code: exec.exit_code,
            exploit_id: exec.exploit_id,
            finished_at: exec.finished_at,
            id: exec.id,
            output: exec.output,
            started_at: exec.started_at,
            target_id: exec.target_id,
            service: target.service,
            target_tick: target.target_tick,
            team: target.team,
        }
    }
}

#[derive(serde::Serialize, PartialEq, Clone)]
pub struct FlagData {
    pub execution_id: i32,
    pub exploit_id: i32,
    pub id: i32,
    pub status: String,
    pub submitted: bool,
    pub text: String,
    pub timestamp: NaiveDateTime,
    pub service: String,
    pub target_tick: i32,
    pub team: String,
}

impl FlagData {
    pub fn from_models(flag: FlagModel, target: TargetModel) -> Self {
        Self {
            execution_id: flag.execution_id,
            exploit_id: flag.exploit_id,
            id: flag.id,
            status: flag.status,
            submitted: flag.submitted,
            text: flag.text,
            timestamp: flag.timestamp,
            service: target.service,
            target_tick: target.target_tick,
            team: target.team,
        }
    }
}

#[derive(serde::Serialize, PartialEq, Debug, Clone, Tabled)]
pub struct ExploitData {
    pub id: i32,
    pub name: String,
    pub service: String,
    pub enabled: bool,
    pub blacklist: String,
    pub pool_size: i32,
}

impl ExploitData {
    pub fn from_model(exploit: ExploitModel) -> Self {
        Self {
            id: exploit.id,
            name: exploit.name,
            service: exploit.service,
            enabled: exploit.enabled,
            blacklist: exploit
                .blacklist
                .into_iter()
                .flatten()
                .collect::<Vec<String>>()
                .join(", "),
            pool_size: exploit.pool_size,
        }
    }
}
