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
pub struct InitAccount;

impl StepContext for InitAccount {
    fn name(&self) -> String {
        "init-account".to_string()
    }

    async fn is_valid(&self, _sdk: &Sdk, state: &State) -> Result<bool, StepError> {
        Ok(state.min_n_implicit_accounts(3))
    }

    async fn build_task(&self, _sdk: &Sdk, state: &State) -> Result<Vec<Task>, StepError> {
        let random_alias = utils::random_alias();
        let account_alias = Alias {
            name: format!("{}-established", random_alias.name),
        };
        let total_signers = utils::random_between(1, 4);
        let required_signers = utils::random_between(1, total_signers);

        let source_aliases = state
            .random_implicit_accounts(vec![], total_signers as usize)
            .into_iter()
            .map(|account| account.alias)
            .collect::<BTreeSet<Alias>>();

        let gas_payer = utils::get_gas_payer(source_aliases.iter(), state);
        let task_settings = TaskSettings::new(source_aliases.clone(), gas_payer);

        Ok(vec![Task::InitAccount(
            task::init_account::InitAccount::builder()
                .target(account_alias)
                .sources(source_aliases)
                .threshold(required_signers)
                .settings(task_settings)
                .build(),
        )])
    }

    fn assert(&self, code: &Code) {
        let is_fatal = code.is_fatal();
        let is_successful = code.is_successful();

        let details = json!({"outcome": code.code()});

        if is_fatal {
            assert_unrechable_step!("Fatal InitAccount", details)
        } else if is_successful {
            assert_always_step!("Done InitAccount", details)
        } else {
            assert_sometimes_step!("Failed InitAccount ", details)
        }
    }
}
