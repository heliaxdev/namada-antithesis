use crate::code::{Code, CodeType};
use crate::context::Ctx;
use crate::error::StepError;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::{assert_always_step, assert_sometimes_step, assert_unreachable_step};

use super::utils;

#[derive(Clone, Debug, Default)]
pub struct ChangeMetadata;

impl StepContext for ChangeMetadata {
    fn name(&self) -> String {
        "change-metadata".to_string()
    }

    async fn is_valid(&self, _ctx: &Ctx, state: &State) -> Result<bool, StepError> {
        Ok(state.min_n_validators(1))
    }

    async fn build_task(&self, _ctx: &Ctx, state: &State) -> Result<Vec<Task>, StepError> {
        let account = state.random_validator(vec![], 1).pop().unwrap();

        let website = utils::get_random_string(15);
        let email = format!("{}@test.com", utils::get_random_string(5));
        let discord = utils::get_random_string(10);
        let description = utils::get_random_string(30);
        let avatar = utils::get_random_string(20);

        let gas_payer = utils::get_gas_payer(account.public_keys.iter(), state);
        let task_settings = TaskSettings::new(account.public_keys, gas_payer);

        Ok(vec![Task::ChangeMetadata(
            task::change_metadata::ChangeMetadata::builder()
                .source(account.alias)
                .website(website)
                .email(email)
                .discord(discord)
                .description(description)
                .avatar(avatar)
                .settings(task_settings)
                .build(),
        )])
    }

    fn assert(&self, code: &Code) {
        match code.code_type() {
            CodeType::Success => assert_always_step!("Done ChangeMetadata", code),
            CodeType::Fatal => assert_unreachable_step!("Fatal ChangeMetadata", code),
            CodeType::Skip => assert_sometimes_step!("Skipped ChangeMetadata", code),
            CodeType::Failed => assert_unreachable_step!("Failed ChangeMetadata", code),
        }
    }
}
