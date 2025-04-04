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
pub struct DeactivateValidator;

impl StepContext for DeactivateValidator {
    fn name(&self) -> String {
        "deactivate-validator".to_string()
    }

    async fn is_valid(&self, _ctx: &Ctx, state: &State) -> Result<bool, StepError> {
        Ok(state.min_n_validators(1))
    }

    async fn build_task(&self, ctx: &Ctx, state: &State) -> Result<Vec<Task>, StepError> {
        let account = state.random_validator(vec![], 1).pop().unwrap();

        let epoch = get_epoch(ctx, retry_config()).await?;

        let gas_payer = utils::get_gas_payer(account.public_keys.iter(), state);
        let task_settings = TaskSettings::new(account.public_keys, gas_payer);

        Ok(vec![Task::DeactivateValidator(
            task::deactivate_validator::DeactivateValidator::builder()
                .target(account.alias)
                .epoch(epoch)
                .settings(task_settings)
                .build(),
        )])
    }

    fn assert(&self, code: &Code) {
        match code.code_type() {
            CodeType::Success => assert_always_step!("Done DeactivateValidator", code),
            CodeType::Fatal => assert_unreachable_step!("Fatal DeactivateValidator", code),
            CodeType::Skip => assert_sometimes_step!("Skipped DeactivateValidator", code),
            CodeType::Failed => assert_unreachable_step!("Failed DeactivateValidator", code),
        }
    }
}
