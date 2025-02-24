use serde_json::json;

use crate::assert_step;
use crate::code::Code;
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::types::Alias;

#[derive(Clone, Debug, Default)]
pub struct ClaimRewards;

impl StepContext for ClaimRewards {
    fn name(&self) -> String {
        "claim-rewards".to_string()
    }

    async fn is_valid(&self, _sdk: &Sdk, state: &State) -> Result<bool, StepError> {
        Ok(state.any_bond())
    }

    async fn build_task(&self, _sdk: &Sdk, state: &mut State) -> Result<Vec<Task>, StepError> {
        let source_bond = state.random_bond();
        let source_account = state.get_account_by_alias(&source_bond.alias);

        let mut task_settings = TaskSettings::new(source_account.public_keys, Alias::faucet());
        task_settings.gas_limit *= 5;

        Ok(vec![Task::ClaimRewards(
            task::claim_rewards::ClaimRewards::builder()
                .source(source_bond.alias)
                .from_validator(source_bond.validator.to_string())
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
            assert_step!("Fatal ClaimRewards", details)
        } else if is_failed {
            assert_step!("Failed ClaimRewards", details)
        } else if is_skipped {
            assert_step!("Skipped ClaimRewards", details)
        } else if is_successful {
            assert_step!("Done ClaimRewards", details)
        } else {
            assert_step!("Unknown Code ClaimRewards ", details)
        }
    }
}
