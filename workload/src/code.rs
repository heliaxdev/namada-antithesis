use crate::executor::StepError;
use crate::state::StateError;
use crate::step::{StepContext, StepType};

pub enum Code {
    Success(StepType),
    Fatal(StepType, StepError),
    ExecutionFailure(StepType, StepError),
    BroadcastFailure(StepType, StepError),
    OtherFailure(StepType, StepError),
    BuildFailure(StepType, StepError),
    InvalidStep(StepType),
    NoTask(StepType),
    EmptyBatch(StepType),
    StateFatal(StateError),
    InitFatal(StepError),
}

impl Code {
    pub fn code(&self) -> i32 {
        match self {
            Code::Success(_) | Code::InvalidStep(_) => 0,
            Code::Fatal(_, _) => 1,
            Code::BuildFailure(_, _) => 2,
            Code::ExecutionFailure(_, _) => 3,
            Code::BroadcastFailure(_, _) => 4,
            Code::OtherFailure(_, _) => 5,
            Code::NoTask(_) => 6,
            Code::EmptyBatch(_) => 7,
            Code::StateFatal(_) => 8,
            Code::InitFatal(_) => 9,
        }
    }

    pub fn step_type(&self) -> Option<&StepType> {
        match self {
            Code::Success(st) => Some(st),
            Code::Fatal(st, _) => Some(st),
            Code::ExecutionFailure(st, _) => Some(st),
            Code::BroadcastFailure(st, _) => Some(st),
            Code::OtherFailure(st, _) => Some(st),
            Code::BuildFailure(st, _) => Some(st),
            Code::InvalidStep(st) => Some(st),
            Code::NoTask(st) => Some(st),
            Code::EmptyBatch(st) => Some(st),
            Code::StateFatal(_) => None,
            Code::InitFatal(_) => None,
        }
    }

    pub fn output_logs(&self) {
        match self {
            Code::Success(step_type) => tracing::info!("Done {step_type} successfully!"),
            Code::Fatal(step_type, reason) => {
                tracing::error!("State check error for {step_type} -> {reason}")
            }
            Code::ExecutionFailure(step_type, reason) => {
                tracing::error!("Transaction execution failure for {step_type} -> {reason}")
            }
            Code::BroadcastFailure(step_type, reason) => tracing::info!(
                "Transaction broadcast failure for {step_type} -> {reason}, waiting for next block"
            ),
            Code::OtherFailure(step_type, reason) => {
                tracing::warn!("Failure for {step_type} -> {reason}")
            }
            Code::InvalidStep(step_type) => {
                tracing::warn!("Invalid step for {step_type}, skipping...")
            }
            Code::NoTask(step_type) => tracing::info!("No task for {step_type}, skipping..."),
            Code::BuildFailure(step_type, reason) => {
                tracing::warn!("Build failure for {step_type} -> {reason}")
            }
            Code::EmptyBatch(step_type) => {
                tracing::error!("Building an empty batch for {step_type}")
            }
            Code::StateFatal(reason) => {
                tracing::error!("State error -> {reason}")
            }
            Code::InitFatal(reason) => {
                tracing::error!("Init error -> {reason}")
            }
        }
    }

    pub fn is_fatal(&self) -> bool {
        matches!(self, Code::Fatal(_, _) | Code::StateFatal(_))
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, Code::ExecutionFailure(_, _))
    }

    pub fn is_skipped(&self) -> bool {
        matches!(self, Code::InvalidStep(_))
    }

    pub fn is_successful(&self) -> bool {
        matches!(self, Code::Success(_))
    }

    pub fn assert(&self) {
        if let Some(step_type) = self.step_type() {
            step_type.assert(self);
        }
    }
}
