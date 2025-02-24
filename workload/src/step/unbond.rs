use namada_sdk::rpc;
use serde_json::json;

use crate::assert_step;
use crate::code::Code;
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::types::Alias;

use super::utils;

#[derive(Clone, Debug, Default)]
pub struct Unbond;

impl StepContext for Unbond {
    fn name(&self) -> String {
        "unbond".to_string()
    }

    async fn is_valid(&self, _sdk: &Sdk, state: &State) -> Result<bool, StepError> {
        Ok(state.any_bond())
    }

    async fn build_task(&self, sdk: &Sdk, state: &mut State) -> Result<Vec<Task>, StepError> {
        let source_bond = state.random_bond();
        let source_account = state.get_account_by_alias(&source_bond.alias);
        let amount = utils::random_between(state, 1, source_bond.amount);

        let current_epoch = rpc::query_epoch(&sdk.namada.client)
            .await
            .map_err(StepError::Rpc)?;

        let mut task_settings = TaskSettings::new(source_account.public_keys, Alias::faucet());
        task_settings.gas_limit *= 3;

        Ok(vec![Task::Unbond(
            task::unbond::Unbond::builder()
                .source(source_account.alias)
                .validator(source_bond.validator)
                .amount(amount)
                .epoch(current_epoch.into())
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
            assert_step!("Fatal Unbond", details)
        } else if is_failed {
            assert_step!("Failed Unbond", details)
        } else if is_skipped {
            assert_step!("Skipped Unbond", details)
        } else if is_successful {
            assert_step!("Done Unbond", details)
        } else {
            assert_step!("Unknown Code Unbond ", details)
        }
    }
}
