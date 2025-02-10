use crate::{
    entities::Alias,
    state::State,
    steps::StepError,
    task::{Task, TaskSettings},
};

use super::utils;

pub async fn build_change_consensus_keys(state: &mut State) -> Result<Vec<Task>, StepError> {
    let account = state.random_validator(vec![], 1).pop().unwrap();

    let random_alias = utils::random_alias(state);
    let consensus_key_alias = format!("{}-consensus", random_alias.name);

    let task_settings = TaskSettings::new(account.public_keys.clone(), Alias::faucet());

    Ok(vec![Task::ChangeConsensusKeys(
        account.alias,
        consensus_key_alias.into(),
        task_settings,
    )])
}
