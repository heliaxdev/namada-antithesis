use namada_sdk::rpc;
use rand::seq::IteratorRandom;

use crate::{
    entities::Alias,
    sdk::namada::Sdk,
    state::State,
    steps::StepError,
    task::{Task, TaskSettings},
};

use super::utils;

pub async fn build_bond(sdk: &Sdk, state: &mut State) -> Result<Vec<Task>, StepError> {
    let client = sdk.namada.clone_client();
    let source_account = state
        .random_account_with_min_balance(vec![])
        .ok_or(StepError::Build("No more accounts".to_string()))?;
    let amount_account = state.get_balance_for(&source_account.alias);
    let amount = utils::random_between(state, 1, amount_account);

    let current_epoch = rpc::query_epoch(&client)
        .await
        .map_err(|e| StepError::Rpc(format!("query epoch: {}", e)))?;
    let validators = rpc::get_all_consensus_validators(&client, current_epoch)
        .await
        .map_err(|e| StepError::Rpc(format!("query consensus validators: {}", e)))?;

    let validator = validators
        .into_iter()
        .map(|v| v.address)
        .choose(&mut state.rng)
        .unwrap(); // safe as there is always at least a validator

    let task_settings = TaskSettings::new(source_account.public_keys, Alias::faucet());

    Ok(vec![Task::Bond(
        source_account.alias,
        validator.to_string(),
        amount,
        current_epoch.into(),
        task_settings,
    )])
}
