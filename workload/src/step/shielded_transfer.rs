use std::collections::BTreeSet;

use serde_json::json;

use crate::code::Code;
use crate::constants::{MAX_BATCH_TX_NUM, MIN_TRANSFER_BALANCE};
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::types::Alias;
use crate::{assert_always_step, assert_sometimes_step, assert_unrechable_step};

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

    async fn build_task(&self, _sdk: &Sdk, state: &State) -> Result<Vec<Task>, StepError> {
        let source_account = state
            .random_masp_account_with_min_balance(vec![], MIN_TRANSFER_BALANCE)
            .ok_or(StepError::BuildTask("No more source accounts".to_string()))?;

        let target_account = state
            .random_payment_address(vec![source_account.alias.clone()])
            .ok_or(StepError::BuildTask("No more target accounts".to_string()))?;
        let amount_account = state.get_shielded_balance_for(&source_account.alias);
        let amount = utils::random_between(1, amount_account / MAX_BATCH_TX_NUM);

        //FIXME Review the signers
        let task_settings = TaskSettings::new(
            BTreeSet::from([source_account.alias.clone()]),
            Alias::faucet(),
        );

        Ok(vec![Task::ShieldedTransfer(
            task::shielded::ShieldedTransfer::builder()
                .source(source_account.alias.spending_key())
                .target(target_account.alias.payment_address())
                .amount(amount)
                .settings(task_settings)
                .build(),
        )])
    }

    fn assert(&self, code: &Code) {
        let is_fatal = code.is_fatal();
        let is_successful = code.is_successful();

        let details = json!({"outcome": code.code()});

        if is_fatal {
            assert_unrechable_step!("Fatal ShieldedTransfer", details)
        } else if is_successful {
            assert_always_step!("Done ShieldedTransfer", details)
        } else {
            assert_sometimes_step!("Failed ShieldedTransfer ", details)
        }
    }
}
