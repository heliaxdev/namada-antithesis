use namada_sdk::rpc;
use serde_json::json;

use crate::code::Code;
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::types::{Alias, ProposalVote};
use crate::{assert_always_step, assert_sometimes_step, assert_unrechable_step};

use super::utils;

#[derive(Clone, Debug, Default)]
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

    async fn build_task(&self, sdk: &Sdk, state: &State) -> Result<Vec<Task>, StepError> {
        let source_bond = state.random_bond();
        let source_account = state.get_account_by_alias(&source_bond.alias);

        let current_epoch = rpc::query_epoch(&sdk.namada.client)
            .await
            .map_err(StepError::Rpc)?;

        let proposal_id = state.random_votable_proposal(current_epoch.0);

        let vote = if utils::coin_flip(0.5) {
            ProposalVote::Yay
        } else if utils::coin_flip(0.5) {
            ProposalVote::Nay
        } else {
            ProposalVote::Abstain
        };

        let mut task_settings = TaskSettings::new(source_account.public_keys, Alias::faucet());
        task_settings.gas_limit *= 5;

        Ok(vec![Task::Vote(
            task::vote::Vote::builder()
                .source(source_account.alias)
                .proposal_id(proposal_id)
                .vote(vote)
                .settings(task_settings)
                .build(),
        )])
    }

    fn assert(&self, code: &Code) {
        let is_fatal = code.is_fatal();
        let is_successful = code.is_successful();

        let details = json!({"outcome": code.code()});

        if is_fatal {
            assert_unrechable_step!("Fatal Vote", details)
        } else if is_successful {
            assert_always_step!("Done Vote", details)
        } else {
            assert_sometimes_step!("Failed Vote ", details)
        }
    }
}
