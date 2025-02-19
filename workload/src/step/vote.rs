use crate::{
    entities::Alias,
    executor::StepError,
    sdk::namada::Sdk,
    state::State,
    step::StepContext,
    task::{self, Task, TaskSettings},
};
use namada_sdk::rpc;

use super::utils;

#[derive(Debug, Default)]
pub struct Vote;

impl StepContext for Vote {
    fn name(&self) -> String {
        "vote".to_string()
    }

    async fn is_valid(&self, sdk: &Sdk, state: &State) -> Result<bool, StepError> {
        let current_epoch = rpc::query_epoch(&sdk.namada.client)
            .await
            .map_err(StepError::Rpc)?;
        Ok(state.any_bond() && state.any_votable_proposal(current_epoch.into()))
    }

    async fn build_task(&self, sdk: &Sdk, state: &mut State) -> Result<Vec<Task>, StepError> {
        let client = sdk.namada.clone_client();
        let source_bond = state.random_bond();
        let source_account = state.get_account_by_alias(&source_bond.alias);

        let current_epoch = rpc::query_epoch(&client).await.map_err(StepError::Rpc)?;

        let proposal_id = state.random_votable_proposal(current_epoch.0);

        let vote = if utils::coin_flip(state, 0.5) {
            "yay"
        } else if utils::coin_flip(state, 0.5) {
            "nay"
        } else {
            "abstain"
        };

        let mut task_settings = TaskSettings::new(source_account.public_keys, Alias::faucet());
        task_settings.gas_limit *= 5;

        Ok(vec![Task::Vote(
            task::vote::Vote::builder()
                .source(source_account.alias)
                .proposal_id(proposal_id)
                .vote(vote.to_string())
                .settings(task_settings)
                .build(),
        )])
    }
}
