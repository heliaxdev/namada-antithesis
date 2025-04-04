use crate::code::{Code, CodeType};
use crate::context::Ctx;
use crate::error::StepError;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::utils::{get_epoch, retry_config};
use crate::{assert_always_step, assert_sometimes_step, assert_unreachable_step};

use super::utils;

#[derive(Clone, Debug, Default)]
pub struct ReactivateValidator;

impl StepContext for ReactivateValidator {
    fn name(&self) -> String {
        "reactivate-validator".to_string()
    }

    async fn is_valid(&self, _ctx: &Ctx, state: &State) -> Result<bool, StepError> {
        Ok(state.min_n_deactivated_validators(1))
    }

    async fn build_task(&self, ctx: &Ctx, state: &State) -> Result<Vec<Task>, StepError> {
        let epoch = get_epoch(ctx, retry_config()).await?;
        let Some(account) = state.random_deactivated_validator(vec![], epoch, 1).pop() else {
            return Ok(vec![]);
        };

        let gas_payer = utils::get_gas_payer(account.public_keys.iter(), state);
        let task_settings = TaskSettings::new(account.public_keys, gas_payer);

        Ok(vec![Task::ReactivateValidator(
            task::reactivate_validator::ReactivateValidator::builder()
                .target(account.alias)
                .settings(task_settings)
                .build(),
        )])
    }

    fn assert(&self, code: &Code) {
        match code.code_type() {
            CodeType::Success => assert_always_step!("Done ReactivateValidator", code),
            CodeType::Fatal => assert_unreachable_step!("Fatal ReactivateValidator", code),
            CodeType::Skip => assert_sometimes_step!("Skipped ReactivateValidator", code),
            CodeType::Failed => assert_unreachable_step!("Failed ReactivateValidator", code),
        }
    }
}
