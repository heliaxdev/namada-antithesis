use rand::{distributions::Standard, prelude::Distribution, seq::SliceRandom, Rng};

use crate::{
    sdk::namada::Sdk,
    state::State,
    steps::StepError,
    task::{Task, TaskSettings},
};

use super::{
    bond::build_bond, claim_rewards::build_claim_rewards, redelegate::build_redelegate,
    shielding::build_shielding, transparent_transfer::build_transparent_transfer,
    unbond::build_unbond,
};

pub async fn build_bond_batch(
    sdk: &Sdk,
    max_size: usize,
    state: &mut State,
) -> Result<Vec<Task>, StepError> {
    _build_batch(sdk, vec![BatchType::Bond], max_size, state).await
}

pub async fn build_random_batch(
    sdk: &Sdk,
    max_size: usize,
    state: &mut State,
) -> Result<Vec<Task>, StepError> {
    _build_batch(
        sdk,
        vec![
            BatchType::TransparentTransfer,
            BatchType::Bond,
            BatchType::Redelegate,
            BatchType::Unbond,
            BatchType::Shielding,
            // BatchType::ClaimRewards, introducing this would be every balance check :(
        ],
        max_size,
        state,
    )
    .await
}

async fn _build_batch(
    sdk: &Sdk,
    possibilities: Vec<BatchType>,
    max_size: usize,
    state: &mut State,
) -> Result<Vec<Task>, StepError> {
    let mut tmp_state = state.clone();

    let mut batch_tasks = vec![];
    for _ in 0..max_size {
        let step: BatchType = possibilities.choose(&mut state.rng).unwrap().to_owned();
        let tasks = match step {
            BatchType::TransparentTransfer => {
                let tasks = build_transparent_transfer(&mut tmp_state).await?;
                tmp_state.update(tasks.clone(), false);
                tasks
            }
            BatchType::Bond => {
                let tasks = build_bond(sdk, &mut tmp_state).await?;
                tmp_state.update(tasks.clone(), false);
                tasks
            }
            BatchType::Redelegate => {
                let tasks = build_redelegate(sdk, &mut tmp_state).await?;
                tmp_state.update(tasks.clone(), false);
                tasks
            }
            BatchType::Unbond => {
                let tasks = build_unbond(sdk, &mut tmp_state).await?;
                tmp_state.update(tasks.clone(), false);
                tasks
            }
            BatchType::Shielding => {
                let tasks = build_shielding(&mut tmp_state).await?;
                tmp_state.update(tasks.clone(), false);
                tasks
            }
            BatchType::ClaimRewards => {
                let tasks = build_claim_rewards(&mut tmp_state);
                tmp_state.update(tasks.clone(), false);
                tasks
            }
        };
        if tasks.is_empty() {
            continue;
        } else {
            tracing::info!("Added {:?} tx type to the batch...", step);
            batch_tasks.extend(tasks);
        }
    }

    let settings = TaskSettings::faucet_batch(batch_tasks.len());

    Ok(vec![Task::Batch(batch_tasks, settings)])
}

#[derive(Debug, Clone)]
enum BatchType {
    TransparentTransfer,
    Redelegate,
    Bond,
    Unbond,
    Shielding,
    ClaimRewards,
}

impl Distribution<BatchType> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> BatchType {
        match rng.gen_range(0..6) {
            0 => BatchType::TransparentTransfer,
            1 => BatchType::Redelegate,
            2 => BatchType::Unbond,
            3 => BatchType::Shielding,
            4 => BatchType::ClaimRewards,
            _ => BatchType::Bond,
        }
    }
}
