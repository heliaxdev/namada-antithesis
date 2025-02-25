use serde_json::json;

use crate::code::Code;
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::types::Alias;
use crate::{assert_always_step, assert_sometimes_step, assert_unrechable_step};

use super::utils;

#[derive(Clone, Debug, Default)]
pub struct ChangeMetadata;

impl StepContext for ChangeMetadata {
    fn name(&self) -> String {
        "change-metadata".to_string()
    }

    async fn is_valid(&self, _sdk: &Sdk, state: &State) -> Result<bool, StepError> {
        Ok(state.min_n_validators(1))
    }

    async fn build_task(&self, _sdk: &Sdk, state: &State) -> Result<Vec<Task>, StepError> {
        let account = state.random_validator(vec![], 1).pop().unwrap();

        let website = utils::get_random_string(15);
        let email = format!("{}@test.com", utils::get_random_string(5));
        let discord = utils::get_random_string(10);
        let description = utils::get_random_string(30);
        let avatar = utils::get_random_string(20);

        let task_settings = TaskSettings::new(account.public_keys.clone(), Alias::faucet());

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
        let is_fatal = code.is_fatal();
        let is_failed = code.is_failed();
        let is_skipped = code.is_skipped();
        let is_successful = code.is_successful();

        let details = json!({"outcome": code.code()});

        if is_fatal {
            assert_unrechable_step!("Fatal ChangeMetadata", details)
        } else if is_failed {
            assert_unrechable_step!("Failed ChangeMetadata", details)
        } else if is_skipped {
            assert_sometimes_step!("Skipped ChangeMetadata", details)
        } else if is_successful {
            assert_always_step!("Done ChangeMetadata", details)
        } else {
            assert_sometimes_step!("Unknown Code ChangeMetadata ", details)
        }
    }
}
