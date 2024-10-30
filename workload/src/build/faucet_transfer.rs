use crate::{
    constants::NATIVE_SCALE,
    state::State,
    task::{Task, TaskSettings},
};

use super::utils;

pub async fn build_faucet_transfer(state: &mut State) -> Vec<Task> {
    let target_account = state.random_account(vec![]);
    let amount = utils::random_between(state, 1000, 2000) * NATIVE_SCALE;

    let task_settings = TaskSettings::faucet();

    vec![Task::FaucetTransfer(
        target_account.alias,
        amount,
        task_settings,
    )]
}
