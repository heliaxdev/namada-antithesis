use crate::{
    entities::Alias,
    state::State,
    steps::StepError,
    task::{Task, TaskSettings},
};

use super::utils;

pub async fn build_transparent_transfer(state: &mut State) -> Result<Vec<Task>, StepError> {
    let source_account = state
        .random_account_with_min_balance(vec![], None)
        .ok_or(StepError::Build("No more accounts".to_string()))?;
    let target_account = state
        .random_account(vec![source_account.alias.clone()])
        .ok_or(StepError::Build("No more accounts".to_string()))?;
    let amount_account = state.get_balance_for(&source_account.alias);
    let amount = utils::random_between(state, 1, amount_account);

    let task_settings = TaskSettings::new(source_account.public_keys, Alias::faucet());

    Ok(vec![Task::TransparentTransfer(
        source_account.alias,
        target_account.alias,
        amount,
        task_settings,
    )])
}
