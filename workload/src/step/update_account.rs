use std::collections::BTreeSet;

use crate::code::{Code, CodeType};
use crate::context::Ctx;
use crate::error::StepError;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::types::Alias;
use crate::{assert_always_step, assert_sometimes_step, assert_unreachable_step};

use super::utils;

#[derive(Clone, Debug, Default)]
pub struct UpdateAccount;

impl StepContext for UpdateAccount {
    fn name(&self) -> String {
        "update-account".to_string()
    }

    async fn is_valid(&self, _ctx: &Ctx, state: &State) -> Result<bool, StepError> {
        Ok(state.min_n_established_accounts(1) && state.min_n_implicit_accounts(3))
    }

    async fn build_task(&self, _ctx: &Ctx, state: &State) -> Result<Vec<Task>, StepError> {
        let account = state.random_established_account(vec![], 1).pop().unwrap();

        let total_signers = utils::random_between(1, 4);
        let required_signers = utils::random_between(1, total_signers);

        let source_aliases = state
            .random_implicit_accounts(vec![], total_signers as usize)
            .into_iter()
            .map(|account| account.alias)
            .collect::<BTreeSet<Alias>>();

        let gas_payer = utils::get_gas_payer(account.public_keys.iter(), state);
        let task_settings = TaskSettings::new(account.public_keys, gas_payer);

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
        match code.code_type() {
            CodeType::Success => assert_always_step!("Done UpdateAccount", code),
            CodeType::Fatal => assert_unreachable_step!("Fatal UpdateAccount", code),
            CodeType::Skip => assert_sometimes_step!("Skipped UpdateAccount", code),
            CodeType::Failed => assert_unreachable_step!("Failed UpdateAccount", code),
        }
    }
}
