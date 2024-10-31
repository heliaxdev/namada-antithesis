use crate::{
    constants::NATIVE_SCALE,
    state::State,
    steps::StepError,
    task::{Task, TaskSettings},
};

use super::utils;

pub async fn build_faucet_transfer(state: &mut State) -> Result<Vec<Task>, StepError> {
    let target_account = state
        .random_account(vec![])
        .ok_or(StepError::Build("No more accounts".to_string()))?;
    let amount = utils::random_between(state, 1000, 2000) * NATIVE_SCALE;

    let task_settings = TaskSettings::faucet();

    Ok(vec![Task::FaucetTransfer(
        target_account.alias,
        amount,
        task_settings,
    )])
}
