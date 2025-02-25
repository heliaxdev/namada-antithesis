use antithesis_sdk::random::AntithesisRng;
use namada_sdk::rpc;
use rand::seq::IteratorRandom;
use serde_json::json;

use crate::code::Code;
use crate::constants::MIN_TRANSFER_BALANCE;
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::types::Alias;
use crate::{assert_always_step, assert_sometimes_step, assert_unrechable_step};

use super::utils;

#[derive(Clone, Debug, Default)]
pub struct Bond;

impl StepContext for Bond {
    fn name(&self) -> String {
        "bond".to_string()
    }

    async fn is_valid(&self, _sdk: &Sdk, state: &State) -> Result<bool, StepError> {
        Ok(state.any_account_with_min_balance(MIN_TRANSFER_BALANCE))
    }

    async fn build_task(&self, sdk: &Sdk, state: &State) -> Result<Vec<Task>, StepError> {
        let client = &sdk.namada.client;
        let source_account = state
            .random_account_with_min_balance(vec![], MIN_TRANSFER_BALANCE)
            .ok_or(StepError::BuildTask("No more accounts".to_string()))?;
        let amount_account = state.get_balance_for(&source_account.alias);
        let amount = utils::random_between(1, amount_account);

        let current_epoch = rpc::query_epoch(client).await.map_err(StepError::Rpc)?;
        let validators = rpc::get_all_consensus_validators(client, current_epoch)
            .await
            .map_err(StepError::Rpc)?;

        let validator = validators
            .into_iter()
            .map(|v| v.address)
            .choose(&mut AntithesisRng)
            .expect("There is always at least a validator");

        let task_settings = TaskSettings::new(source_account.public_keys, Alias::faucet());

        Ok(vec![Task::Bond(
            task::bond::Bond::builder()
                .source(source_account.alias)
                .validator(validator.to_string())
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
            assert_unrechable_step!("Fatal Bond", details)
        } else if is_failed {
            assert_unrechable_step!("Failed Bond", details)
        } else if is_skipped {
            assert_sometimes_step!("Skipped Bond", details)
        } else if is_successful {
            assert_always_step!("Done Bond", details)
        } else {
            assert_sometimes_step!("Unknown Code Bond ", details)
        }
    }
}
