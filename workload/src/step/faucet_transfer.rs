use serde_json::json;

use crate::code::Code;
use crate::constants::{FAUCET_AMOUNT, NATIVE_SCALE};
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::{assert_always_step, assert_sometimes_step, assert_unrechable_step};

#[derive(Clone, Debug, Default)]
pub struct FaucetTransfer;

impl StepContext for FaucetTransfer {
    fn name(&self) -> String {
        "faucet-transfer".to_string()
    }

    async fn is_valid(&self, _sdk: &Sdk, state: &State) -> Result<bool, StepError> {
        Ok(state.any_account())
    }

    async fn build_task(&self, _sdk: &Sdk, state: &State) -> Result<Vec<Task>, StepError> {
        let target_account = state
            .random_account(vec![])
            .ok_or(StepError::BuildTask("No more accounts".to_string()))?;
        let amount = FAUCET_AMOUNT * NATIVE_SCALE;

        let task_settings = TaskSettings::faucet();

        Ok(vec![Task::FaucetTransfer(
            task::faucet_transfer::FaucetTransfer::builder()
                .target(target_account.alias)
                .amount(amount)
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
            assert_unrechable_step!("Fatal FaucetTransfer", details)
        } else if is_failed {
            assert_unrechable_step!("Failed FaucetTransfer", details)
        } else if is_skipped {
            assert_sometimes_step!("Skipped FaucetTransfer", details)
        } else if is_successful {
            assert_always_step!("Done FaucetTransfer", details)
        } else {
            assert_sometimes_step!("Unknown Code FaucetTransfer ", details)
        }
    }
}
