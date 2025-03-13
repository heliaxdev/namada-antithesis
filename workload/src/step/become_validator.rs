use namada_sdk::dec::Dec;
use serde_json::json;

use crate::code::Code;
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::task::{self, Task, TaskSettings};
use crate::{assert_always_step, assert_sometimes_step, assert_unrechable_step};

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

    async fn build_task(&self, _sdk: &Sdk, state: &State) -> Result<Vec<Task>, StepError> {
        let commission_rate = utils::random_between::<u64>(0, 100);
        let commission_rate = Dec::new(commission_rate as i128, 2).unwrap();

        let commission_rate_change = utils::random_between::<u64>(0, 100);
        let commission_rate_change = Dec::new(commission_rate_change as i128, 2).unwrap();

        let random_alias = utils::random_alias();
        let consensus_key_alias = format!("{}-consensus", random_alias.name);
        let eth_cold_key_alias = format!("{}-eth-cold", random_alias.name);
        let eth_hot_key_alias = format!("{}-eth-hot", random_alias.name);
        let protocol_key_alias = format!("{}-protocol", random_alias.name);

        let account = state.random_enstablished_account(vec![], 1).pop().unwrap();

        let gas_payer = utils::get_gas_payer(account.public_keys.iter(), state);
        let task_settings = TaskSettings::new(account.public_keys, gas_payer);

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
        let is_successful = code.is_successful();

        let details = json!({"outcome": code.code()});

        if is_fatal {
            assert_unrechable_step!("Fatal BecomeValidator", details)
        } else if is_successful {
            assert_always_step!("Done BecomeValidator", details)
        } else {
            assert_sometimes_step!("Failed Code BecomeValidator ", details)
        }
    }
}
