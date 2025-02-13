use crate::{
    entities::Alias,
    sdk::namada::Sdk,
    state::State,
    steps::StepError,
    task::{Task, TaskSettings},
};
use namada_sdk::rpc;

use super::utils;

pub async fn build_vote(sdk: &Sdk, state: &mut State) -> Result<Vec<Task>, StepError> {
    let client = sdk.namada.clone_client();
    let source_bond = state.random_bond();
    let source_account = state.get_account_by_alias(&source_bond.alias);

    let current_epoch = rpc::query_epoch(&client)
        .await
        .map_err(|e| StepError::Rpc(format!("query epoch: {}", e)))?;

    let proposal_id = state.random_votable_proposal(current_epoch.0);

    let vote = if utils::coin_flip(state, 0.5) {
        "yay"
    } else {
        if utils::coin_flip(state, 0.5) {
            "nay"
        } else {
            "abstain"
        }
    };

    let mut task_settings = TaskSettings::new(source_account.public_keys, Alias::faucet());
    task_settings.gas_limit *= 5;

    Ok(vec![Task::Vote(
        source_account.alias,
        proposal_id,
        vote.to_string(),
        task_settings,
    )])
}
