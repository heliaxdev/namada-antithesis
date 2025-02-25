use std::collections::BTreeSet;

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
pub struct UpdateAccount;

impl StepContext for UpdateAccount {
    fn name(&self) -> String {
        "update-account".to_string()
    }

    async fn is_valid(&self, _sdk: &Sdk, state: &State) -> Result<bool, StepError> {
        Ok(state.min_n_enstablished_accounts(1) && state.min_n_implicit_accounts(3))
    }

    async fn build_task(&self, _sdk: &Sdk, state: &State) -> Result<Vec<Task>, StepError> {
        let account = state.random_enstablished_account(vec![], 1).pop().unwrap();

        let total_signers = utils::random_between(1, 4);
        let required_signers = utils::random_between(1, total_signers);

        let source_aliases = state
            .random_implicit_accounts(vec![], total_signers as usize)
            .into_iter()
            .map(|account| account.alias)
            .collect::<BTreeSet<Alias>>();

        let task_settings = TaskSettings::new(account.public_keys.clone(), Alias::faucet());

        Ok(vec![Task::UpdateAccount(
            task::update_account::UpdateAccount::builder()
                .target(account.alias)
                .sources(source_aliases)
                .threshold(required_signers)
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
            assert_unrechable_step!("Fatal UpdateAccount", details)
        } else if is_failed {
            assert_unrechable_step!("Failed UpdateAccount", details)
        } else if is_skipped {
            assert_sometimes_step!("Skipped UpdateAccount", details)
        } else if is_successful {
            assert_always_step!("Done UpdateAccount", details)
        } else {
            assert_sometimes_step!("Unknown Code UpdateAccount ", details)
        }
    }
}
