use namada_sdk::rpc;
use serde_json::json;

use crate::assert_step;
use crate::code::Code;
use crate::constants::PROPOSAL_DEPOSIT;
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::types::Alias;

use super::utils;

#[derive(Clone, Debug, Default)]
pub struct DefaultProposal;

impl StepContext for DefaultProposal {
    fn name(&self) -> String {
        "default-proposal".to_string()
    }

    async fn is_valid(&self, _sdk: &Sdk, state: &State) -> Result<bool, StepError> {
        Ok(state.any_account_with_min_balance(PROPOSAL_DEPOSIT))
    }

    async fn build_task(&self, sdk: &Sdk, state: &mut State) -> Result<Vec<Task>, StepError> {
        let client = sdk.namada.clone_client();
        let source_account = state
            .random_account_with_min_balance(vec![], PROPOSAL_DEPOSIT)
            .ok_or(StepError::BuildTask("No more accounts".to_string()))?;

        let current_epoch = rpc::query_epoch(&client).await.map_err(StepError::Rpc)?;

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
            task::default_proposal::DefaultProposal::builder()
                .source(source_account.alias)
                .start_epoch(start_epoch)
                .end_epoch(end_epoch)
                .grace_epoch(grace_epoch)
                .settings(task_settings)
                .build(),
        )])
    }

    fn assert(&self, code: &Code) {
        let is_fatal = code.is_fatal();
        let is_failed = code.is_failed();
        let is_skipped = code.is_skipped();
        let is_successful = code.is_successful();

        let details = json!({"outcome": code.code()});

        if is_fatal {
            assert_step!("Fatal DefaultProposal", details)
        } else if is_failed {
            assert_step!("Failed DefaultProposal", details)
        } else if is_skipped {
            assert_step!("Skipped DefaultProposal", details)
        } else if is_successful {
            assert_step!("Done DefaultProposal", details)
        } else {
            assert_step!("Unknown Code DefaultProposal ", details)
        }
    }
}
