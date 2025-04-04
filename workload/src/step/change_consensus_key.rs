use crate::code::{Code, CodeType};
use crate::context::Ctx;
use crate::error::StepError;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::{assert_always_step, assert_sometimes_step, assert_unreachable_step};

use super::utils;

#[derive(Clone, Debug, Default)]
pub struct ChangeConsensusKey;

impl StepContext for ChangeConsensusKey {
    fn name(&self) -> String {
        "change-consensus-key".to_string()
    }

    async fn is_valid(&self, _ctx: &Ctx, state: &State) -> Result<bool, StepError> {
        Ok(state.min_n_validators(1))
    }

    async fn build_task(&self, _ctx: &Ctx, state: &State) -> Result<Vec<Task>, StepError> {
        let account = state.random_validator(vec![], 1).pop().unwrap();

        let random_alias = utils::random_alias();
        let consensus_key_alias = format!("{}-consensus", random_alias.name);

        let gas_payer = utils::get_gas_payer(account.public_keys.iter(), state);
        let task_settings = TaskSettings::new(account.public_keys, gas_payer);

        Ok(vec![Task::ChangeConsensusKey(
            task::change_consensus_key::ChangeConsensusKey::builder()
                .source(account.alias)
                .consensus_alias(consensus_key_alias.into())
                .settings(task_settings)
                .build(),
        )])
    }

    fn assert(&self, code: &Code) {
        match code.code_type() {
            CodeType::Success => assert_always_step!("Done ChangeConsensusKey", code),
            CodeType::Fatal => assert_unreachable_step!("Fatal ChangeConsensusKey", code),
            CodeType::Skip => assert_sometimes_step!("Skipped ChangeConsensusKey", code),
            CodeType::Failed => assert_unreachable_step!("Failed ChangeConsensusKey", code),
        }
    }
}
