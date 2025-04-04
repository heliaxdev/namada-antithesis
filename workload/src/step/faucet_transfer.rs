use crate::code::{Code, CodeType};
use crate::constants::FAUCET_AMOUNT;
use crate::context::Ctx;
use crate::error::StepError;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::{assert_always_step, assert_unreachable_step};

#[derive(Clone, Debug, Default)]
pub struct FaucetTransfer;

impl StepContext for FaucetTransfer {
    fn name(&self) -> String {
        "faucet-transfer".to_string()
    }

    async fn is_valid(&self, _ctx: &Ctx, state: &State) -> Result<bool, StepError> {
        Ok(state.any_account())
    }

    async fn build_task(&self, _ctx: &Ctx, state: &State) -> Result<Vec<Task>, StepError> {
        let target_account = state
            .random_account(vec![])
            .ok_or(StepError::BuildTask("No more accounts".to_string()))?;

        let task_settings = TaskSettings::faucet();

        Ok(vec![Task::FaucetTransfer(
            task::faucet_transfer::FaucetTransfer::builder()
                .target(target_account.alias)
                .amount(FAUCET_AMOUNT)
                .settings(task_settings)
                .build(),
        )])
    }

    fn assert(&self, code: &Code) {
        match code.code_type() {
            CodeType::Success => assert_always_step!("Done FaucetTransfer", code),
            CodeType::Fatal => assert_unreachable_step!("Fatal FaucetTransfer", code),
            CodeType::Skip => assert_unreachable_step!("Skipped FaucetTransfer", code),
            CodeType::Failed => assert_unreachable_step!("Failed FaucetTransfer", code),
        }
    }
}
