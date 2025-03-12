use std::collections::BTreeSet;

use serde_json::json;

use crate::code::Code;
use crate::constants::{DEFAULT_FEE, MAX_BATCH_TX_NUM, MIN_TRANSFER_BALANCE};
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::step::utils::coin_flip;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::{assert_always_step, assert_sometimes_step, assert_unrechable_step};

use super::utils;

#[derive(Clone, Debug, Default)]
pub struct Unshielding;

impl StepContext for Unshielding {
    fn name(&self) -> String {
        "unshielding".to_string()
    }

    async fn is_valid(&self, _sdk: &Sdk, state: &State) -> Result<bool, StepError> {
        Ok(
            state.at_least_masp_account_with_minimal_balance(1, MIN_TRANSFER_BALANCE)
                && state.at_least_accounts(1),
        )
    }

    async fn build_task(&self, _sdk: &Sdk, state: &State) -> Result<Vec<Task>, StepError> {
        let source_account = state
            .random_masp_account_with_min_balance(vec![], MIN_TRANSFER_BALANCE)
            .ok_or(StepError::BuildTask("No more accounts".to_string()))?;

        let target_account = state
            .random_account(vec![source_account.alias.clone()])
            .ok_or(StepError::BuildTask("No more accounts".to_string()))?;
        let amount_account = state.get_shielded_balance_for(&source_account.alias);
        let amount = utils::random_between(1, amount_account / MAX_BATCH_TX_NUM);

        let transparent_source_balance = state.get_balance_for(&source_account.alias.base());
        let disposable_gas_payer = transparent_source_balance < DEFAULT_FEE || coin_flip(0.5);
        let task_settings = TaskSettings::new_with_payer(
            BTreeSet::from([source_account.alias.base()]),
            if disposable_gas_payer {
                source_account.alias.spending_key()
            } else {
                source_account.alias.base()
            },
        );

        Ok(vec![Task::Unshielding(
            task::unshielding::Unshielding::builder()
                .source(source_account.alias.spending_key())
                .target(target_account.alias)
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
            assert_unrechable_step!("Fatal Unshielding", details)
        } else if is_successful {
            assert_always_step!("Done Unshielding", details)
        } else {
            assert_sometimes_step!("Failed Unshielding ", details)
        }
    }
}
