use crate::error::{CheckError, StepError, TaskError};
use crate::state::StateError;
use crate::step::{StepContext, StepType};

pub enum Code {
    Success(StepType),
    // Fatal failures
    Fatal(StepType, CheckError),
    StateFatal(StateError),
    InitFatal(StepError),
    ConfigFatal(String),
    // No execution
    Skip(StepType),
    NoTask(StepType),
    // Other failures
    StepFailure(StepType, StepError),
    TaskFailure(StepType, TaskError),
    CheckFailure(StepType, CheckError),
}

pub enum CodeType {
    Success,
    Fatal,
    Skip,
    Failed,
}

impl Code {
    pub fn code(&self) -> i32 {
        match self {
            Code::Fatal(_, _) => 1,
            Code::StateFatal(_) => 2,
            Code::InitFatal(_) => 3,
            Code::ConfigFatal(_) => 4,
            // system state is as expected
            _ => 0,
        }
    }

    pub fn step_type(&self) -> Option<&StepType> {
        match self {
            Code::Success(st) => Some(st),
            Code::Fatal(st, _) => Some(st),
            Code::StateFatal(_) => None,
            Code::InitFatal(_) => None,
            Code::ConfigFatal(_) => None,
            Code::Skip(st) => Some(st),
            Code::NoTask(st) => Some(st),
            Code::StepFailure(st, _) => Some(st),
            Code::TaskFailure(st, _) => Some(st),
            Code::CheckFailure(st, _) => Some(st),
        }
    }

    pub fn output_logs(&self) {
        match self {
            Code::Success(step_type) => tracing::info!("Done {step_type} successfully!"),
            Code::Fatal(step_type, reason) => {
                tracing::error!("State check error for {step_type} -> {reason}")
            }
            Code::StepFailure(step_type, reason) => {
                tracing::error!("Step failure for {step_type} -> {reason}")
            }
            Code::TaskFailure(step_type, reason) => {
                tracing::error!("Task failure for {step_type} -> {reason}")
            }
            Code::CheckFailure(step_type, reason) => {
                tracing::error!("Check failure for {step_type} -> {reason}")
            }
            Code::Skip(step_type) => {
                tracing::warn!("Invalid step for {step_type}, skipping...")
            }
            Code::NoTask(step_type) => tracing::info!("No task for {step_type}, skipping..."),
            Code::StateFatal(reason) => {
                tracing::error!("State error -> {reason}")
            }
            Code::InitFatal(reason) => {
                tracing::error!("Init error -> {reason}")
            }
            Code::ConfigFatal(reason) => {
                tracing::error!("Config error -> {reason}")
            }
        }
    }

    pub fn code_type(&self) -> CodeType {
        match self {
            Code::Success(_) => CodeType::Success,
            Code::Fatal(_, _) | Code::StateFatal(_) | Code::InitFatal(_) | Code::ConfigFatal(_) => {
                CodeType::Fatal
            }
            Code::Skip(_) | Code::NoTask(_) => CodeType::Skip,
            _ => CodeType::Failed,
        }
    }

    pub fn details(&self) -> serde_json::Value {
        let (outcome, error) = match self {
            Code::Success(_) => ("Success", Default::default()),
            Code::Fatal(_, e) => ("Fatal failure", e.to_string()),
            Code::StateFatal(e) => ("Fatal state failure", e.to_string()),
            Code::InitFatal(e) => ("Fatal init failure", e.to_string()),
            Code::ConfigFatal(e) => ("Fatal config failure", e.clone()),
            Code::Skip(_) => ("Skipped step", Default::default()),
            Code::NoTask(_) => ("No task", Default::default()),
            Code::StepFailure(_, e) => ("Step failure", e.to_string()),
            Code::TaskFailure(_, e) => ("Task failure", e.to_string()),
            Code::CheckFailure(_, e) => ("Check failure", e.to_string()),
        };
        serde_json::json!({
            "outcome": outcome,
            "error": error
        })
    }

    pub fn assert(&self) {
        if let Some(step_type) = self.step_type() {
            step_type.assert(self);
        }
    }
}
