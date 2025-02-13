
use crate::{
    constants::PROPOSAL_DEPOSIT,
    entities::Alias,
    sdk::namada::Sdk,
    state::State,
    steps::StepError,
    task::{Task, TaskSettings},
};
use namada_sdk::rpc;

use super::utils;

pub async fn build_default_proposal(sdk: &Sdk, state: &mut State) -> Result<Vec<Task>, StepError> {
    let client = sdk.namada.clone_client();
    let source_account = state
        .random_account_with_min_balance(vec![], Some(PROPOSAL_DEPOSIT))
        .ok_or(StepError::Build("No more accounts".to_string()))?;

    let current_epoch = rpc::query_epoch(&client)
        .await
        .map_err(|e| StepError::Rpc(format!("query epoch: {}", e)))?;

    let gov_prams = rpc::query_governance_parameters(&client).await;

    let start_epoch = utils::random_between(
        state,
        current_epoch.0 + 2,
        current_epoch.0 + gov_prams.max_proposal_latency,
    );
    let end_epoch = utils::random_between(
        state,
        start_epoch + gov_prams.min_proposal_voting_period,
        start_epoch + gov_prams.max_proposal_period - 5,
    );
    let grace_epoch = utils::random_between(
        state,
        end_epoch + gov_prams.min_proposal_grace_epochs,
        end_epoch + 5,
    );

    let task_settings = TaskSettings::new(source_account.public_keys, Alias::faucet());

    Ok(vec![Task::DefaultProposal(
        source_account.alias,
        start_epoch,
        end_epoch,
        grace_epoch,
        task_settings,
    )])
}
