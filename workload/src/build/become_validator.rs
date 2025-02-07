use std::collections::BTreeSet;

use namada_sdk::dec::Dec;

use crate::{
    entities::Alias,
    state::{Account, State},
    steps::StepError,
    task::{Task, TaskSettings},
};

use super::utils;

pub async fn build_become_validator(state: &mut State) -> Result<Vec<Task>, StepError> {
    let commission_rate = utils::random_between::<u64>(state, 0, 100);
    let commission_rate = Dec::new(commission_rate as i128, 2).unwrap();

    let commission_rate_change = utils::random_between::<u64>(state, 0, 100);
    let commission_rate_change = Dec::new(commission_rate_change as i128, 2).unwrap();

    let random_alias = utils::random_alias(state);
    let consensus_key_alias = format!("{}-{}", random_alias.name, "consensus".to_string());
    let eth_cold_key_alias = format!("{}-{}", random_alias.name, "eth-cold".to_string());
    let eth_hot_key_alias = format!("{}-{}", random_alias.name, "eth-hot".to_string());
    let protocol_key_alias = format!("{}-{}", random_alias.name, "protocol".to_string());

    let account = state.random_enstablished_account(vec![], 1).pop().unwrap();

    let task_settings = TaskSettings::new(account.public_keys.clone(), Alias::faucet());

    Ok(vec![Task::BecomeValidator(
        account.alias,
        consensus_key_alias.into(),
        eth_cold_key_alias.into(),
        eth_hot_key_alias.into(),
        protocol_key_alias.into(),
        commission_rate,
        commission_rate_change,
        task_settings,
    )])
}
