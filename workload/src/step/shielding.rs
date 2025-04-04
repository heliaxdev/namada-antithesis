use crate::code::{Code, CodeType};
use crate::constants::{MAX_BATCH_TX_NUM, MIN_TRANSFER_BALANCE};
use crate::context::Ctx;
use crate::error::{StepError, TaskError};
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::utils::{get_masp_epoch, retry_config};
use crate::{assert_always_step, assert_sometimes_step, assert_unreachable_step};

use super::utils;

#[derive(Clone, Debug, Default)]
pub struct Shielding;

impl StepContext for Shielding {
    fn name(&self) -> String {
        "shielding".to_string()
    }

    async fn is_valid(&self, _ctx: &Ctx, state: &State) -> Result<bool, StepError> {
        Ok(state.any_account_with_min_balance(MIN_TRANSFER_BALANCE))
    }

    async fn build_task(&self, ctx: &Ctx, state: &State) -> Result<Vec<Task>, StepError> {
        let source_account = state
            .random_account_with_min_balance(vec![], MIN_TRANSFER_BALANCE)
            .ok_or(StepError::BuildTask("No more accounts".to_string()))?;
        let epoch = get_masp_epoch(ctx, retry_config()).await?;
        let target_account = state
            .random_payment_address(vec![])
            .ok_or(StepError::BuildTask("No more accounts".to_string()))?;
        let amount_account = state.get_balance_for(&source_account.alias);
        let amount = utils::random_between(1, amount_account / MAX_BATCH_TX_NUM);

        let gas_payer = utils::get_gas_payer(source_account.public_keys.iter(), state);
        let task_settings = TaskSettings::new(source_account.public_keys, gas_payer);

        Ok(vec![Task::Shielding(
            task::shielding::Shielding::builder()
                .source(source_account.alias)
                .target(target_account.alias.payment_address())
                .amount(amount)
                .epoch(epoch)
                .settings(task_settings)
                .build(),
        )])
    }

    fn assert(&self, code: &Code) {
        match code.code_type() {
            CodeType::Success => assert_always_step!("Done Shielding", code),
            CodeType::Fatal => assert_unreachable_step!("Fatal Shielding", code),
            CodeType::Skip => assert_sometimes_step!("Skipped Shielding", code),
            CodeType::Failed
                if matches!(
                    code,
                    Code::TaskFailure(_, TaskError::InvalidShielded { .. })
                ) =>
            {
                assert_sometimes_step!("Invalid Shielding", code)
            }
            _ => assert_unreachable_step!("Failed Shielding", code),
        }
    }
}
