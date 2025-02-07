use std::collections::BTreeSet;

use crate::{
    entities::Alias,
    state::State,
    steps::StepError,
    task::{Task, TaskSettings},
};

use super::utils;

pub async fn build_unshielding(state: &mut State) -> Result<Vec<Task>, StepError> {
    let source_account = state
        .random_masp_account_with_min_balance(vec![], 2)
        .ok_or(StepError::Build("No more accounts".to_string()))?;

    let target_account = state
        .random_account(vec![source_account.alias.clone()])
        .ok_or(StepError::Build("No more accounts".to_string()))?;
    let amount_account = state.get_shielded_balance_for(&source_account.payment_address);
    let amount = utils::random_between(state, 1, amount_account);

    //FIXME Review the signers
    let task_settings = TaskSettings::new(
        BTreeSet::from([source_account.alias.clone()]),
        Alias::faucet(),
    );

    Ok(vec![Task::Unshielding(
        source_account.spending_key,
        target_account.alias,
        amount,
        task_settings,
    )])
}
