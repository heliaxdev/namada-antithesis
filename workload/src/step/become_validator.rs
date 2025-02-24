use namada_sdk::dec::Dec;
use serde_json::json;

use crate::assert_step;
use crate::code::Code;
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::task::{self, Task, TaskSettings};
use crate::types::Alias;

use super::utils;
use super::StepContext;

#[derive(Clone, Debug, Default)]
pub struct BecomeValidator;

impl StepContext for BecomeValidator {
    fn name(&self) -> String {
        "become-validator".to_string()
    }

    async fn is_valid(&self, _sdk: &Sdk, state: &State) -> Result<bool, StepError> {
        Ok(state.min_n_enstablished_accounts(1))
    }

    async fn build_task(&self, _sdk: &Sdk, state: &mut State) -> Result<Vec<Task>, StepError> {
        let commission_rate = utils::random_between::<u64>(state, 0, 100);
        let commission_rate = Dec::new(commission_rate as i128, 2).unwrap();

        let commission_rate_change = utils::random_between::<u64>(state, 0, 100);
        let commission_rate_change = Dec::new(commission_rate_change as i128, 2).unwrap();

        let random_alias = utils::random_alias(state);
        let consensus_key_alias = format!("{}-consensus", random_alias.name);
        let eth_cold_key_alias = format!("{}-eth-cold", random_alias.name);
        let eth_hot_key_alias = format!("{}-eth-hot", random_alias.name);
        let protocol_key_alias = format!("{}-protocol", random_alias.name);

        let account = state.random_enstablished_account(vec![], 1).pop().unwrap();

        let task_settings = TaskSettings::new(account.public_keys.clone(), Alias::faucet());

        Ok(vec![Task::BecomeValidator(
            task::become_validator::BecomeValidator::builder()
                .source(account.alias)
                .consensus_alias(consensus_key_alias.into())
                .eth_cold_alias(eth_cold_key_alias.into())
                .eth_hot_alias(eth_hot_key_alias.into())
                .protocol_alias(protocol_key_alias.into())
                .commission_rate(commission_rate)
                .commission_max_change(commission_rate_change)
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
            assert_step!("Fatal BecomeValidator", details)
        } else if is_failed {
            assert_step!("Failed BecomeValidator", details)
        } else if is_skipped {
            assert_step!("Skipped BecomeValidator", details)
        } else if is_successful {
            assert_step!("Done BecomeValidator", details)
        } else {
            assert_step!("Unknown Code BecomeValidator ", details)
        }
    }
}
