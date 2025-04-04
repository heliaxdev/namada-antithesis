use crate::code::{Code, CodeType};
use crate::constants::{MAX_BATCH_TX_NUM, MIN_TRANSFER_BALANCE};
use crate::context::Ctx;
use crate::error::StepError;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::{assert_always_step, assert_sometimes_step, assert_unreachable_step};

use super::utils;

#[derive(Clone, Debug, Default)]
pub struct TransparentTransfer;

impl StepContext for TransparentTransfer {
    fn name(&self) -> String {
        "transparent-transfer".to_string()
    }

    async fn is_valid(&self, _ctx: &Ctx, state: &State) -> Result<bool, StepError> {
        Ok(state.at_least_accounts(2) && state.any_account_can_make_transfer())
    }

    async fn build_task(&self, _ctx: &Ctx, state: &State) -> Result<Vec<Task>, StepError> {
        let source_account = state
            .random_account_with_min_balance(vec![], MIN_TRANSFER_BALANCE)
            .ok_or(StepError::BuildTask("No more accounts".to_string()))?;
        let target_account = state
            .random_account(vec![source_account.alias.clone()])
            .ok_or(StepError::BuildTask("No more accounts".to_string()))?;
        let amount_account = state.get_balance_for(&source_account.alias);
        let amount = utils::random_between(1, amount_account / MAX_BATCH_TX_NUM);

        let gas_payer = utils::get_gas_payer(source_account.public_keys.iter(), state);
        let task_settings = TaskSettings::new(source_account.public_keys, gas_payer);

        Ok(vec![Task::TransparentTransfer(
            task::transparent_transfer::TransparentTransfer::builder()
                .source(source_account.alias)
                .target(target_account.alias)
                .amount(amount)
                .settings(task_settings)
                .build(),
        )])
    }

    fn assert(&self, code: &Code) {
        match code.code_type() {
            CodeType::Success => assert_always_step!("Done TransparentTransfer", code),
            CodeType::Fatal => assert_unreachable_step!("Fatal TransparentTransfer", code),
            CodeType::Skip => assert_sometimes_step!("Skipped TransparentTransfer", code),
            CodeType::Failed => assert_unreachable_step!("Failed TransparentTransfer", code),
        }
    }
}
