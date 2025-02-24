use std::collections::BTreeSet;

use serde_json::json;

use crate::assert_step;
use crate::code::Code;
use crate::constants::MIN_TRANSFER_BALANCE;
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::types::Alias;

use super::utils;

#[derive(Clone, Debug, Default)]
pub struct ShieldedTransfer;

impl StepContext for ShieldedTransfer {
    fn name(&self) -> String {
        "shielded-transfer".to_string()
    }

    async fn is_valid(&self, _sdk: &Sdk, state: &State) -> Result<bool, StepError> {
        Ok(state.at_least_masp_accounts(2)
            && state.at_least_masp_account_with_minimal_balance(1, MIN_TRANSFER_BALANCE))
    }

    async fn build_task(&self, _sdk: &Sdk, state: &mut State) -> Result<Vec<Task>, StepError> {
        let source_account = state
            .random_masp_account_with_min_balance(vec![], MIN_TRANSFER_BALANCE)
            .ok_or(StepError::BuildTask("No more source accounts".to_string()))?;

        let target_account = state
            .random_payment_address(vec![])
            .ok_or(StepError::BuildTask("No more target accounts".to_string()))?;
        let amount_account = state.get_shielded_balance_for(&source_account.payment_address);
        let amount = utils::random_between(state, 1, amount_account);

        //FIXME Review the signers
        let task_settings = TaskSettings::new(
            BTreeSet::from([source_account.alias.clone()]),
            Alias::faucet(),
        );

        Ok(vec![Task::ShieldedTransfer(
            task::shielded::ShieldedTransfer::builder()
                .source(source_account.spending_key)
                .target(target_account.payment_address)
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
            assert_step!("Fatal ShieldedTransfer", details)
        } else if is_failed {
            assert_step!("Failed ShieldedTransfer", details)
        } else if is_skipped {
            assert_step!("Skipped ShieldedTransfer", details)
        } else if is_successful {
            assert_step!("Done ShieldedTransfer", details)
        } else {
            assert_step!("Unknown Code ShieldedTransfer ", details)
        }
    }
}
