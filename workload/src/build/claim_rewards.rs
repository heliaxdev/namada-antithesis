use crate::{
    entities::Alias,
    state::State,
    task::{Task, TaskSettings},
};

pub fn build_claim_rewards(state: &mut State) -> Vec<Task> {
    let source_bond = state.random_bond();
    let source_account = state.get_account_by_alias(&source_bond.alias);

    let mut task_settings = TaskSettings::new(source_account.public_keys, Alias::faucet());
    task_settings.gas_limit *= 5;

    vec![Task::ClaimRewards(
        source_bond.alias,
        source_bond.validator.to_string(),
        task_settings,
    )]
}
