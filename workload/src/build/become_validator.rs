use namada_sdk::dec::Dec;

use crate::{
    entities::Alias,
    state::State,
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
    let consensus_key_alias = format!("{}-consensus", random_alias.name);
    let eth_cold_key_alias = format!("{}-eth-cold", random_alias.name);
    let eth_hot_key_alias = format!("{}-eth-hot", random_alias.name);
    let protocol_key_alias = format!("{}-protocol", random_alias.name);

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
