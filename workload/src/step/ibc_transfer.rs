use crate::code::{Code, CodeType};
use crate::constants::{
    COSMOS_TOKEN, MAX_BATCH_TX_NUM, MAX_COSMOS_TRANSFER_AMOUNT, MIN_TRANSFER_BALANCE,
};
use crate::context::Ctx;
use crate::error::StepError;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::types::Alias;
use crate::{assert_always_step, assert_sometimes_step, assert_unreachable_step};

use super::utils;

#[derive(Clone, Debug, Default)]
pub struct IbcTransferSend;

impl StepContext for IbcTransferSend {
    fn name(&self) -> String {
        "ibc-transfer-send".to_string()
    }

    async fn is_valid(&self, _ctx: &Ctx, state: &State) -> Result<bool, StepError> {
        Ok(state.any_account_can_make_transfer())
    }

    async fn build_task(&self, ctx: &Ctx, state: &State) -> Result<Vec<Task>, StepError> {
        let source_account = state
            .random_account_with_min_balance(vec![], MIN_TRANSFER_BALANCE)
            .ok_or(StepError::BuildTask("No more accounts".to_string()))?;
        let target_account = ctx.cosmos.account.to_string();
        // TODO: non-native token
        let token = Alias::nam().name;
        let amount_account = state.get_balance_for(&source_account.alias);
        let amount = utils::random_between(1, amount_account / MAX_BATCH_TX_NUM);

        let gas_payer = utils::get_gas_payer(source_account.public_keys.iter(), state);
        let task_settings = TaskSettings::new(source_account.public_keys, gas_payer);

        Ok(vec![Task::IbcTransferSend(
            task::ibc_transfer::IbcTransferSend::builder()
                .source(source_account.alias)
                .receiver(target_account.into())
                .amount(amount)
                .token(token)
                .src_channel_id(ctx.namada_channel_id.clone())
                .dest_channel_id(ctx.cosmos_channel_id.clone())
                .settings(task_settings)
                .build(),
        )])
    }

    fn assert(&self, code: &Code) {
        match code.code_type() {
            CodeType::Success => assert_always_step!("Done IbcTransferSend", code),
            CodeType::Fatal => assert_unreachable_step!("Fatal IbcTransferSend", code),
            CodeType::Skip => assert_sometimes_step!("Skipped IbcTransferSend", code),
            CodeType::Failed => assert_unreachable_step!("Failed IbcTransferSend", code),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct IbcTransferRecv;

impl StepContext for IbcTransferRecv {
    fn name(&self) -> String {
        "ibc-transfer-recv".to_string()
    }

    async fn is_valid(&self, _ctx: &Ctx, state: &State) -> Result<bool, StepError> {
        Ok(state.any_account())
    }

    async fn build_task(&self, ctx: &Ctx, state: &State) -> Result<Vec<Task>, StepError> {
        let source_account = ctx.cosmos.account.to_string();
        let target_account = state
            .random_account(vec![])
            .ok_or(StepError::BuildTask("No more accounts".to_string()))?;
        // TODO: nam back
        let token = COSMOS_TOKEN.to_string();
        let amount = utils::random_between(1, MAX_COSMOS_TRANSFER_AMOUNT);

        // task settings is not used, but required
        let task_settings = TaskSettings::faucet();

        Ok(vec![Task::IbcTransferRecv(
            task::ibc_transfer::IbcTransferRecv::builder()
                .sender(source_account.into())
                .target(target_account.alias)
                .amount(amount)
                .token(token)
                .src_channel_id(ctx.cosmos_channel_id.clone())
                .dest_channel_id(ctx.namada_channel_id.clone())
                .settings(task_settings)
                .build(),
        )])
    }

    fn assert(&self, code: &Code) {
        match code.code_type() {
            CodeType::Success => assert_always_step!("Done IbcTransferSend", code),
            CodeType::Fatal => assert_unreachable_step!("Fatal IbcTransferSend", code),
            CodeType::Skip => assert_sometimes_step!("Skipped IbcTransferSend", code),
            CodeType::Failed => assert_unreachable_step!("Failed IbcTransferSend", code),
        }
    }
}
