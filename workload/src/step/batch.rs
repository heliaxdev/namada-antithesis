use std::collections::HashSet;

use antithesis_sdk::random::AntithesisRng;
use rand::seq::SliceRandom;

use crate::code::{Code, CodeType};
use crate::constants::MAX_BATCH_TX_NUM;
use crate::constants::MIN_TRANSFER_BALANCE;
use crate::context::Ctx;
use crate::error::{StepError, TaskError};
use crate::state::State;
use crate::step::{StepContext, StepType};
use crate::task::{self, Task, TaskSettings};
use crate::{assert_always_step, assert_sometimes_step, assert_unreachable_step};

#[derive(Clone, Debug, Default)]
pub struct BatchBond;

impl StepContext for BatchBond {
    fn name(&self) -> String {
        "batch-bond".to_string()
    }

    async fn is_valid(&self, _ctx: &Ctx, state: &State) -> Result<bool, StepError> {
        Ok(state.min_n_account_with_min_balance(3, MIN_TRANSFER_BALANCE))
    }

    async fn build_task(&self, ctx: &Ctx, state: &State) -> Result<Vec<Task>, StepError> {
        Box::pin(build_batch(
            ctx,
            vec![StepType::Bond(Default::default())],
            MAX_BATCH_TX_NUM,
            state,
        ))
        .await
    }

    fn assert(&self, code: &Code) {
        match code.code_type() {
            CodeType::Success => assert_always_step!("Done BatchBond", code),
            CodeType::Fatal => assert_unreachable_step!("Fatal BatchBond", code),
            CodeType::Skip => assert_sometimes_step!("Skipped BatchBond", code),
            CodeType::Failed => assert_unreachable_step!("Failed BatchBond", code),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct BatchRandom;

impl StepContext for BatchRandom {
    fn name(&self) -> String {
        "batch-random".to_string()
    }

    async fn is_valid(&self, _ctx: &Ctx, state: &State) -> Result<bool, StepError> {
        Ok(state.min_n_account_with_min_balance(3, MIN_TRANSFER_BALANCE) && state.min_bonds(3))
    }

    async fn build_task(&self, ctx: &Ctx, state: &State) -> Result<Vec<Task>, StepError> {
        Box::pin(build_batch(
            ctx,
            vec![
                StepType::TransparentTransfer(Default::default()),
                StepType::Bond(Default::default()),
                StepType::Redelegate(Default::default()),
                StepType::Unbond(Default::default()),
                StepType::Shielding(Default::default()),
                StepType::Unshielding(Default::default()),
            ],
            MAX_BATCH_TX_NUM,
            state,
        ))
        .await
    }

    fn assert(&self, code: &Code) {
        match code.code_type() {
            CodeType::Success => assert_always_step!("Done BatchRandom", code),
            CodeType::Fatal => assert_unreachable_step!("Fatal BatchRandom", code),
            CodeType::Skip => assert_sometimes_step!("Skipped BatchRandom", code),
            CodeType::Failed
                if matches!(
                    code,
                    Code::TaskFailure(_, TaskError::InvalidShielded { .. })
                ) =>
            {
                assert_sometimes_step!("Invalid BatchRandom including shielded actions", code)
            }
            _ => assert_unreachable_step!("Failed BatchRandom", code),
        }
    }
}

async fn build_batch(
    ctx: &Ctx,
    possibilities: Vec<StepType>,
    max_size: u64,
    state: &State,
) -> Result<Vec<Task>, StepError> {
    let mut batch_tasks = vec![];
    for _ in 0..max_size {
        let step = possibilities
            .choose(&mut AntithesisRng)
            .expect("at least one StepType should exist");
        let tasks = step.build_task(ctx, state).await.unwrap_or_default();
        if !tasks.is_empty() {
            tracing::info!("Added {step} to the batch...");
            batch_tasks.extend(tasks);
        }
    }

    let mut shielded_sources = HashSet::new();
    let mut redelegated_targets = HashSet::new();
    let batch_tasks: Vec<Task> = batch_tasks
        .into_iter()
        .filter(|task| {
            match task {
                // if the shielded source has been already used,
                // remove the task to avoid spending the same masp note
                Task::ShieldedTransfer(inner) => shielded_sources.insert(inner.source().clone()),
                Task::Unshielding(inner) => shielded_sources.insert(inner.source().clone()),
                // if the redelegated source validator has been already used as a target,
                // remove the task to avoid chained redelegation
                Task::Redelegate(inner) => {
                    let (from, to) = (inner.from_validator(), inner.to_validator());
                    if redelegated_targets.contains(from) {
                        false
                    } else {
                        redelegated_targets.insert(to.clone());
                        true
                    }
                }
                _ => true,
            }
        })
        .collect();

    if batch_tasks.is_empty() {
        return Ok(vec![]);
    }

    let settings = TaskSettings::faucet_batch(batch_tasks.len());

    Ok(vec![Task::Batch(
        task::batch::Batch::builder()
            .tasks(batch_tasks)
            .settings(settings)
            .build(),
    )])
}
