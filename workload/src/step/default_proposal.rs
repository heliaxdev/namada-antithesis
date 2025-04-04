use namada_sdk::rpc;

use crate::code::{Code, CodeType};
use crate::constants::PROPOSAL_DEPOSIT;
use crate::context::Ctx;
use crate::error::StepError;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::utils::{get_epoch, retry_config};
use crate::{assert_always_step, assert_sometimes_step, assert_unreachable_step};

use super::utils;

#[derive(Clone, Debug, Default)]
pub struct DefaultProposal;

impl StepContext for DefaultProposal {
    fn name(&self) -> String {
        "default-proposal".to_string()
    }

    async fn is_valid(&self, _ctx: &Ctx, state: &State) -> Result<bool, StepError> {
        Ok(state.any_account_with_min_balance(PROPOSAL_DEPOSIT))
    }

    async fn build_task(&self, ctx: &Ctx, state: &State) -> Result<Vec<Task>, StepError> {
        let client = &ctx.namada.client;
        let source_account = state
            .random_account_with_min_balance(vec![], PROPOSAL_DEPOSIT)
            .ok_or(StepError::BuildTask("No more accounts".to_string()))?;

        let current_epoch = get_epoch(ctx, retry_config()).await?;

        let gov_prams = rpc::query_governance_parameters(client).await;

        let start_epoch = utils::random_between(
            current_epoch + 2,
            current_epoch + gov_prams.max_proposal_latency,
        );
        let end_epoch = utils::random_between(
            start_epoch + gov_prams.min_proposal_voting_period,
            start_epoch + gov_prams.max_proposal_period - 5,
        );
        let grace_epoch = utils::random_between(
            end_epoch + gov_prams.min_proposal_grace_epochs,
            end_epoch + 5,
        );

        let gas_payer = utils::get_gas_payer(source_account.public_keys.iter(), state);
        let task_settings = TaskSettings::new(source_account.public_keys, gas_payer);

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
        match code.code_type() {
            CodeType::Success => assert_always_step!("Done DefaultProposal", code),
            CodeType::Fatal => assert_unreachable_step!("Fatal DefaultProposal", code),
            CodeType::Skip => assert_sometimes_step!("Skipped DefaultProposal", code),
            CodeType::Failed => assert_unreachable_step!("Failed DefaultProposal", code),
        }
    }
}
