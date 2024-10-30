use rand::{distributions::Standard, prelude::Distribution, Rng};

use crate::{sdk::namada::Sdk, state::State, steps::StepError, task::Task};

use super::{bond::build_bond, transparent_transfer::build_transparent_transfer};

pub async fn build_batch(
    sdk: &Sdk,
    bond: bool,
    transfer: bool,
    max_size: usize,
    state: &mut State,
) -> Result<Vec<Task>, StepError> {
    let mut tmp_state = state.clone();

    let mut batch_tasks = vec![];
    for _ in 0..max_size {
        let step: BatchType = rand::random();
        let tasks = match step {
            BatchType::TransparentTransfer => {
                if !transfer {
                    vec![]
                } else {
                    let tasks = build_transparent_transfer(&mut tmp_state).await?;
                    tmp_state.update(tasks.clone(), false);
                    tasks
                }
            }
            BatchType::Bond => {
                if !bond {
                    vec![]
                } else {
                    let tasks = build_bond(sdk, &mut tmp_state).await?;
                    tmp_state.update(tasks.clone(), false);
                    tasks
                }
            }
        };
        batch_tasks.extend(tasks);
    }

    Ok(batch_tasks)
}

#[derive(Debug, Clone)]
enum BatchType {
    TransparentTransfer,
    Bond,
}

impl Distribution<BatchType> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> BatchType {
        match rng.gen_range(0..1) {
            0 => BatchType::TransparentTransfer,
            _ => BatchType::Bond,
        }
    }
}
