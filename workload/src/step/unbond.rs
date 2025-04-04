use crate::code::{Code, CodeType};
use crate::constants::MAX_BATCH_TX_NUM;
use crate::context::Ctx;
use crate::error::StepError;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::utils::{get_epoch, retry_config};
use crate::{assert_always_step, assert_sometimes_step, assert_unreachable_step};

use super::utils;

#[derive(Clone, Debug, Default)]
pub struct Unbond;

impl StepContext for Unbond {
    fn name(&self) -> String {
        "unbond".to_string()
    }

    async fn is_valid(&self, _ctx: &Ctx, state: &State) -> Result<bool, StepError> {
        Ok(state.any_bond())
    }

    async fn build_task(&self, ctx: &Ctx, state: &State) -> Result<Vec<Task>, StepError> {
        let current_epoch = get_epoch(ctx, retry_config()).await?;
        let Some(source_bond) = state.random_bond(current_epoch) else {
            return Ok(vec![]);
        };
        let source_account = state.get_account_by_alias(&source_bond.alias);
        let amount = utils::random_between(1, source_bond.amount / MAX_BATCH_TX_NUM);

        let gas_payer = utils::get_gas_payer(source_account.public_keys.iter(), state);
        let mut task_settings = TaskSettings::new(source_account.public_keys, gas_payer);
        task_settings.gas_limit *= 3;

        Ok(vec![Task::Unbond(
            task::unbond::Unbond::builder()
                .source(source_account.alias)
                .validator(source_bond.validator)
                .amount(amount)
                .epoch(current_epoch)
                .settings(task_settings)
                .build(),
        )])
    }

    fn assert(&self, code: &Code) {
        match code.code_type() {
            CodeType::Success => assert_always_step!("Done Unbond", code),
            CodeType::Fatal => assert_unreachable_step!("Fatal Unbond", code),
            CodeType::Skip => assert_sometimes_step!("Skipped Unbond", code),
            CodeType::Failed => assert_unreachable_step!("Failed Unbond", code),
        }
    }
}
