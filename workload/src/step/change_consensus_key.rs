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
pub struct ChangeConsensusKey;

impl StepContext for ChangeConsensusKey {
    fn name(&self) -> String {
        "change-consensus-key".to_string()
    }

    async fn is_valid(&self, _sdk: &Sdk, state: &State) -> Result<bool, StepError> {
        Ok(state.min_n_validators(1))
    }

    async fn build_task(&self, _sdk: &Sdk, state: &State) -> Result<Vec<Task>, StepError> {
        let account = state.random_validator(vec![], 1).pop().unwrap();

        let random_alias = utils::random_alias();
        let consensus_key_alias = format!("{}-consensus", random_alias.name);

        let task_settings = TaskSettings::new(account.public_keys.clone(), Alias::faucet());

        Ok(vec![Task::ChangeConsensusKey(
            task::change_consensus_key::ChangeConsensusKey::builder()
                .source(account.alias)
                .consensus_alias(consensus_key_alias.into())
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
            assert_unrechable_step!("Fatal ChangeConsensusKey", details)
        } else if is_failed {
            assert_unrechable_step!("Failed ChangeConsensusKey", details)
        } else if is_skipped {
            assert_sometimes_step!("Skipped ChangeConsensusKey", details)
        } else if is_successful {
            assert_always_step!("Done ChangeConsensusKey", details)
        } else {
            assert_sometimes_step!("Unknown Code ChangeConsensusKey ", details)
        }
    }
}
