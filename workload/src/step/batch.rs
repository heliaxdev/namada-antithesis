use rand::seq::SliceRandom;
use serde_json::json;

use crate::assert_step;
use crate::code::Code;
use crate::constants::MAX_BATCH_TX_NUM;
use crate::constants::MIN_TRANSFER_BALANCE;
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::step::{StepContext, StepType};
use crate::task::{self, Task, TaskContext, TaskSettings};

#[derive(Clone, Debug, Default)]
pub struct BatchBond;

impl StepContext for BatchBond {
    fn name(&self) -> String {
        "batch-bond".to_string()
    }

    async fn is_valid(&self, _sdk: &Sdk, state: &State) -> Result<bool, StepError> {
        Ok(state.min_n_account_with_min_balance(3, MIN_TRANSFER_BALANCE))
    }

    async fn build_task(&self, sdk: &Sdk, state: &mut State) -> Result<Vec<Task>, StepError> {
        Box::pin(build_batch(
            sdk,
            vec![StepType::Bond(Default::default())],
            MAX_BATCH_TX_NUM,
            state,
        ))
        .await
    }

    fn assert(&self, code: &Code) {
        let is_fatal = code.is_fatal();
        let is_failed = code.is_failed();
        let is_skipped = code.is_skipped();
        let is_successful = code.is_successful();

        let details = json!({"outcome": code.code()});

        if is_fatal {
            assert_step!("Fatal BatchBond", details)
        } else if is_failed {
            assert_step!("Failed BatchBond", details)
        } else if is_skipped {
            assert_step!("Skipped BatchBond", details)
        } else if is_successful {
            assert_step!("Done BatchBond", details)
        } else {
            assert_step!("Unknown Code BatchBond ", details)
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct BatchRandom;

impl StepContext for BatchRandom {
    fn name(&self) -> String {
        "batch-random".to_string()
    }

    async fn is_valid(&self, _sdk: &Sdk, state: &State) -> Result<bool, StepError> {
        Ok(state.min_n_account_with_min_balance(3, MIN_TRANSFER_BALANCE) && state.min_bonds(3))
    }

    async fn build_task(&self, sdk: &Sdk, state: &mut State) -> Result<Vec<Task>, StepError> {
        Box::pin(build_batch(
            sdk,
            vec![
                StepType::TransparentTransfer(Default::default()),
                StepType::Bond(Default::default()),
                StepType::Redelegate(Default::default()),
                StepType::Unbond(Default::default()),
                StepType::Shielding(Default::default()),
                StepType::Unshielding(Default::default()),
                // StepType::ClaimRewards, introducing this would fail every balance check :(
            ],
            MAX_BATCH_TX_NUM,
            state,
        ))
        .await
    }

    fn assert(&self, code: &Code) {
        let is_fatal = code.is_fatal();
        let is_failed = code.is_failed();
        let is_skipped = code.is_skipped();
        let is_successful = code.is_successful();

        let details = json!({"outcome": code.code()});

        if is_fatal {
            assert_step!("Fatal BatchRandom", details)
        } else if is_failed {
            assert_step!("Failed BatchRandom", details)
        } else if is_skipped {
            assert_step!("Skipped BatchRandom", details)
        } else if is_successful {
            assert_step!("Done BatchRandom", details)
        } else {
            assert_step!("Unknown Code BatchRandom ", details)
        }
    }
}

async fn build_batch(
    sdk: &Sdk,
    possibilities: Vec<StepType>,
    max_size: u64,
    state: &mut State,
) -> Result<Vec<Task>, StepError> {
    let mut tmp_state = state.clone();

    let mut batch_tasks = vec![];
    for _ in 0..max_size {
        let step = possibilities
            .choose(&mut state.rng)
            .expect("at least one StepType should exist");
        let tasks = step.build_task(sdk, &mut tmp_state).await?;
        for task in &tasks {
            task.update_state(&mut tmp_state, false);
        }
        if !tasks.is_empty() {
            tracing::info!("Added {step} to the batch...");
            batch_tasks.extend(tasks);
        }
    }

    if batch_tasks.is_empty() {
        return Err(StepError::EmptyBatch);
    }

    let settings = TaskSettings::faucet_batch(batch_tasks.len());

    Ok(vec![Task::Batch(
        task::batch::Batch::builder()
            .tasks(batch_tasks)
            .settings(settings)
            .build(),
    )])
}
