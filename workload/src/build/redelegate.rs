use std::str::FromStr;

use namada_sdk::{address::Address, rpc};
use rand::seq::IteratorRandom;

use crate::{
    entities::Alias,
    sdk::namada::Sdk,
    state::State,
    steps::StepError,
    task::{Task, TaskSettings},
};

use super::utils;

pub async fn build_redelegate(sdk: &Sdk, state: &mut State) -> Result<Vec<Task>, StepError> {
    let client = sdk.namada.clone_client();
    let source_bond = state.random_bond();
    let source_account = state.get_account_by_alias(&source_bond.alias);
    let amount = utils::random_between(state, 1, source_bond.amount);

    let current_epoch = rpc::query_epoch(&client)
        .await
        .map_err(|e| StepError::Rpc(format!("query epoch: {}", e)))?.next().next();
    let validators = rpc::get_all_consensus_validators(&client, current_epoch)
        .await
        .map_err(|e| StepError::Rpc(format!("query consensus validators: {}", e)))?;

    let source_bond_validator_address = Address::from_str(&source_bond.validator).unwrap();

    let to_validator = validators
        .into_iter()
        .filter_map(|v| if v.address.eq(&source_bond_validator_address) { None } else { Some(v.address) })
        .choose(&mut state.rng)
        .unwrap(); // safe as there is always at least a validator

    let mut task_settings = TaskSettings::new(source_account.public_keys, Alias::faucet());
    task_settings.gas_limit *= 3;

    Ok(vec![Task::Redelegate(
        source_account.alias,
        source_bond.validator.to_string(),
        to_validator.to_string(),
        amount,
        current_epoch.into(),
        task_settings,
    )])
}
