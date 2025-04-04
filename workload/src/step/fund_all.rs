use crate::code::{Code, CodeType};
use crate::constants::FAUCET_AMOUNT;
use crate::context::Ctx;
use crate::error::StepError;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::{assert_always_step, assert_unreachable_step};

#[derive(Clone, Debug, Default)]
pub struct FundAll;

impl StepContext for FundAll {
    fn name(&self) -> String {
        "fund-all".to_string()
    }

    async fn is_valid(&self, _ctx: &Ctx, state: &State) -> Result<bool, StepError> {
        Ok(!state.accounts.is_empty())
    }

    async fn build_task(&self, _ctx: &Ctx, state: &State) -> Result<Vec<Task>, StepError> {
        let tasks: Vec<_> = state
            .accounts
            .keys()
            .map(|alias| {
                Task::FaucetTransfer(
                    task::faucet_transfer::FaucetTransfer::builder()
                        .target(alias.clone())
                        .amount(FAUCET_AMOUNT)
                        .settings(TaskSettings::faucet())
                        .build(),
                )
            })
            .collect();

        let settings = TaskSettings::faucet_batch(tasks.len());
        Ok(vec![Task::Batch(
            task::batch::Batch::builder()
                .tasks(tasks)
                .settings(settings)
                .build(),
        )])
    }

    fn assert(&self, code: &Code) {
        match code.code_type() {
            CodeType::Success => assert_always_step!("Done FundAll", code),
            CodeType::Fatal => assert_unreachable_step!("Fatal FundAll", code),
            CodeType::Skip => assert_unreachable_step!("Skipped FundAll", code),
            CodeType::Failed => assert_unreachable_step!("Failed FundAll", code),
        }
    }
}
