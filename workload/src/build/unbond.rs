use namada_sdk::rpc;

use crate::{
    entities::Alias,
    sdk::namada::Sdk,
    state::State,
    steps::StepError,
    task::{Task, TaskSettings},
};

use super::utils;

pub async fn build_unbond(sdk: &Sdk, state: &mut State) -> Result<Vec<Task>, StepError> {
    let client = sdk.namada.clone_client();
    let source_bond = state.random_bond();
    let source_account = state.get_account_by_alias(&source_bond.alias);
    let amount = utils::random_between(state, 1, source_bond.amount);

    let current_epoch = rpc::query_epoch(&client)
        .await
        .map_err(|e| StepError::Rpc(format!("query epoch: {}", e)))?;

    let mut task_settings = TaskSettings::new(source_account.public_keys, Alias::faucet());
    task_settings.gas_limit *= 3;

    Ok(vec![Task::Unbond(
        source_account.alias,
        source_bond.validator,
        amount,
        current_epoch.next().next().next().next().next().next().into(),
        task_settings,
    )])
}
