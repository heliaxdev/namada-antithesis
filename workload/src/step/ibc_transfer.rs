use std::collections::BTreeSet;

use crate::code::{Code, CodeType};
use crate::constants::DEFAULT_FEE;
use crate::constants::{
    COSMOS_TOKEN, MAX_BATCH_TX_NUM, MAX_COSMOS_TRANSFER_AMOUNT, MIN_TRANSFER_BALANCE,
};
use crate::context::Ctx;
use crate::error::{StepError, TaskError};
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::types::Alias;
use crate::utils::{get_masp_epoch, ibc_denom, is_native_denom, retry_config};
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
        let (source_account, denom) = state
            .random_account_with_ibc_balance(vec![])
            .filter(|_| utils::coin_flip(0.5))
            .map(|account| (account, ibc_denom(&ctx.namada_channel_id, COSMOS_TOKEN)))
            .or_else(|| {
                state
                    .random_account_with_min_balance(vec![], MIN_TRANSFER_BALANCE)
                    .map(|account| (account, Alias::nam().name))
            })
            .ok_or(StepError::BuildTask("No more accounts".to_string()))?;
        let target_account = ctx.cosmos.account.to_string();
        let amount_account = if is_native_denom(&denom) {
            state.get_balance_for(&source_account.alias)
        } else {
            state.get_ibc_balance_for(&source_account.alias, &denom)
        };
        let amount = utils::random_between(1, amount_account / MAX_BATCH_TX_NUM);

        let gas_payer = utils::get_gas_payer(source_account.public_keys.iter(), state);
        let task_settings = TaskSettings::new(source_account.public_keys, gas_payer);

        Ok(vec![Task::IbcTransferSend(
            task::ibc_transfer::IbcTransferSend::builder()
                .source(source_account.alias)
                .receiver(target_account.into())
                .amount(amount)
                .denom(denom)
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
            CodeType::Failed if matches!(code, Code::TaskFailure(_, TaskError::IbcTransfer(_))) => {
                assert_sometimes_step!("Failed IbcTransferSend (acceptable)", code)
            }
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
        let source = ctx.cosmos.account.to_string().into();
        let target_account = state
            .random_account(vec![])
            .ok_or(StepError::BuildTask("No more accounts".to_string()))?;
        let foreign_balance = state.get_foreign_balance_for(&source);
        let (denom, max_amount) = if foreign_balance > 0 && utils::coin_flip(0.5) {
            (
                ibc_denom(&ctx.cosmos_channel_id, &Alias::nam().name),
                foreign_balance / MAX_BATCH_TX_NUM,
            )
        } else {
            (COSMOS_TOKEN.to_string(), MAX_COSMOS_TRANSFER_AMOUNT)
        };
        let amount = utils::random_between(1, max_amount);

        // task settings is not used, but required
        let task_settings = TaskSettings::faucet();

        Ok(vec![Task::IbcTransferRecv(
            task::ibc_transfer::IbcTransferRecv::builder()
                .sender(source)
                .target(target_account.alias)
                .amount(amount)
                .denom(denom)
                .src_channel_id(ctx.cosmos_channel_id.clone())
                .dest_channel_id(ctx.namada_channel_id.clone())
                .settings(task_settings)
                .build(),
        )])
    }

    fn assert(&self, code: &Code) {
        match code.code_type() {
            CodeType::Success => assert_always_step!("Done IbcTransferRecv", code),
            CodeType::Fatal => assert_unreachable_step!("Fatal IbcTransferRecv", code),
            CodeType::Skip => assert_sometimes_step!("Skipped IbcTransferRecv", code),
            CodeType::Failed if matches!(code, Code::TaskFailure(_, TaskError::IbcTransfer(_))) => {
                assert_sometimes_step!("Failed IbcTransferRecv (acceptable)", code)
            }
            CodeType::Failed => assert_unreachable_step!("Failed IbcTransferRecv", code),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct IbcShieldingTransfer;

impl StepContext for IbcShieldingTransfer {
    fn name(&self) -> String {
        "ibc-shielding-transfer".to_string()
    }

    async fn is_valid(&self, _ctx: &Ctx, state: &State) -> Result<bool, StepError> {
        Ok(state.any_account())
    }

    async fn build_task(&self, ctx: &Ctx, state: &State) -> Result<Vec<Task>, StepError> {
        let source = ctx.cosmos.account.to_string().into();
        let target_account = state
            .random_payment_address(vec![])
            .ok_or(StepError::BuildTask("No more accounts".to_string()))?;
        let foreign_balance = state.get_foreign_balance_for(&source);
        let (denom, max_amount) = if foreign_balance > 0 && utils::coin_flip(0.5) {
            (
                ibc_denom(&ctx.cosmos_channel_id, &Alias::nam().name),
                foreign_balance / MAX_BATCH_TX_NUM,
            )
        } else {
            (COSMOS_TOKEN.to_string(), MAX_COSMOS_TRANSFER_AMOUNT)
        };
        let amount = utils::random_between(1, max_amount);

        // task settings is not used, but required
        let task_settings = TaskSettings::faucet();

        Ok(vec![Task::IbcShieldingTransfer(
            task::ibc_transfer::IbcShieldingTransfer::builder()
                .sender(source)
                .target(target_account.alias.payment_address())
                .amount(amount)
                .denom(denom)
                .src_channel_id(ctx.cosmos_channel_id.clone())
                .dest_channel_id(ctx.namada_channel_id.clone())
                .settings(task_settings)
                .build(),
        )])
    }

    fn assert(&self, code: &Code) {
        match code.code_type() {
            CodeType::Success => assert_always_step!("Done IbcShieldingTransfer", code),
            CodeType::Fatal => assert_unreachable_step!("Fatal IbcShieldingTransfer", code),
            CodeType::Skip => assert_sometimes_step!("Skipped IbcShieldingTransfer", code),
            CodeType::Failed if matches!(code, Code::TaskFailure(_, TaskError::IbcTransfer(_))) => {
                assert_sometimes_step!("Failed IbcShieldingTransfer (acceptable)", code)
            }
            CodeType::Failed
                if matches!(
                    code,
                    Code::TaskFailure(_, TaskError::InvalidShielded { .. })
                ) =>
            {
                assert_sometimes_step!("Invalid IbcShieldingTransfer", code)
            }
            _ => assert_unreachable_step!("Failed IbcShieldingTransfer", code),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct IbcUnshieldingTransfer;

impl StepContext for IbcUnshieldingTransfer {
    fn name(&self) -> String {
        "ibc-unshielding-transfer".to_string()
    }

    async fn is_valid(&self, _ctx: &Ctx, state: &State) -> Result<bool, StepError> {
        Ok(state.any_account_can_make_transfer())
    }

    async fn build_task(&self, ctx: &Ctx, state: &State) -> Result<Vec<Task>, StepError> {
        let Some((source_account, denom)) = state
            .random_masp_account_with_ibc_balance(vec![])
            .filter(|_| utils::coin_flip(0.5))
            .map(|account| (account, ibc_denom(&ctx.namada_channel_id, COSMOS_TOKEN)))
            .or_else(|| {
                state
                    .random_masp_account_with_min_balance(vec![], MIN_TRANSFER_BALANCE)
                    .map(|account| (account, Alias::nam().name))
            })
        else {
            return Ok(vec![]);
        };
        let target_account = ctx.cosmos.account.to_string();
        let amount_account = if is_native_denom(&denom) {
            state.get_shielded_balance_for(&source_account.alias)
        } else {
            state.get_ibc_balance_for(&source_account.alias.spending_key(), &denom)
        };
        let amount = utils::random_between(1, amount_account / MAX_BATCH_TX_NUM);

        let transparent_source_balance = state.get_balance_for(&source_account.alias.base());
        let shielded_source_balance =
            state.get_shielded_balance_for(&source_account.alias.spending_key());
        if transparent_source_balance < DEFAULT_FEE && shielded_source_balance < DEFAULT_FEE {
            // Insufficient balance for the fee
            return Ok(vec![]);
        }
        let disposable_gas_payer = transparent_source_balance < DEFAULT_FEE
            || (shielded_source_balance >= DEFAULT_FEE && utils::coin_flip(0.5));
        let task_settings = TaskSettings::new(
            BTreeSet::from([source_account.alias.base()]),
            if disposable_gas_payer {
                source_account.alias.spending_key()
            } else {
                source_account.alias.base()
            },
        );

        let epoch = get_masp_epoch(ctx, retry_config()).await?;

        Ok(vec![Task::IbcUnshieldingTransfer(
            task::ibc_transfer::IbcUnshieldingTransfer::builder()
                .source(source_account.alias.spending_key())
                .receiver(target_account.into())
                .amount(amount)
                .denom(denom)
                .src_channel_id(ctx.namada_channel_id.clone())
                .dest_channel_id(ctx.cosmos_channel_id.clone())
                .epoch(epoch)
                .settings(task_settings)
                .build(),
        )])
    }

    fn assert(&self, code: &Code) {
        match code.code_type() {
            CodeType::Success => assert_always_step!("Done IbcUnshieldingTransfer", code),
            CodeType::Fatal => assert_unreachable_step!("Fatal IbcUnshieldingTransfer", code),
            CodeType::Skip => assert_sometimes_step!("Skipped IbcUnshieldingTransfer", code),
            CodeType::Failed if matches!(code, Code::TaskFailure(_, TaskError::IbcTransfer(_))) => {
                assert_sometimes_step!("Failed IbcUnshieldingTransfer (acceptable)", code)
            }
            CodeType::Failed
                if matches!(
                    code,
                    Code::TaskFailure(_, TaskError::InvalidShielded { .. })
                ) =>
            {
                assert_sometimes_step!("Invalid IbcUnshieldingTransfer", code)
            }
            _ => assert_unreachable_step!("Failed IbcUnshieldingTransfer", code),
        }
    }
}
