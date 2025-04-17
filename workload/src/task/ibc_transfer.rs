use cosmrs::Any;
use namada_sdk::args::{self, InputAmount, TxBuilder};
use namada_sdk::ibc::core::host::types::identifiers::{ChannelId, PortId};
use namada_sdk::masp_primitives::transaction::components::sapling::builder::RngBuildParams;
use namada_sdk::signing::SigningTxData;
use namada_sdk::tx::data::GasLimit;
use namada_sdk::tx::Tx;
use namada_sdk::Namada;
use namada_sdk::{token, TransferSource};
use rand::rngs::OsRng;
use typed_builder::TypedBuilder;

use crate::check::{self, Check};
use crate::context::Ctx;
use crate::error::TaskError;
use crate::state::State;
use crate::task::{TaskContext, TaskSettings};
use crate::types::{Alias, Amount, Height};
use crate::utils::{
    base_denom, build_cosmos_ibc_transfer, cosmos_denom_hash, get_balance, ibc_denom,
    ibc_token_address, is_native_denom, RetryConfig,
};

#[derive(Clone, Debug, TypedBuilder)]
pub struct IbcTransferSend {
    source: Alias,
    receiver: Alias,
    denom: String,
    amount: Amount,
    src_channel_id: ChannelId,
    dest_channel_id: ChannelId,
    settings: TaskSettings,
}

impl TaskContext for IbcTransferSend {
    fn name(&self) -> String {
        "ibc-transfer-send".to_string()
    }

    fn summary(&self) -> String {
        format!(
            "ibc-transfer-send/{}/{}/'{}'/{}",
            self.source.name, self.receiver.name, self.denom, self.amount
        )
    }

    fn task_settings(&self) -> Option<&TaskSettings> {
        Some(&self.settings)
    }

    async fn build_tx(&self, ctx: &Ctx) -> Result<(Tx, Vec<SigningTxData>, args::Tx), TaskError> {
        let mut bparams = RngBuildParams::new(OsRng);

        let wallet = ctx.namada.wallet.read().await;

        let source_address = wallet
            .find_address(&self.source.name)
            .ok_or_else(|| TaskError::Wallet(format!("No source address: {}", self.source.name)))?;
        let token_address = if is_native_denom(&self.denom) {
            wallet
                .find_address(&self.denom)
                .ok_or_else(|| {
                    TaskError::Wallet(format!("No native token address: {}", self.denom))
                })?
                .into_owned()
        } else {
            ibc_token_address(&self.denom)
        };
        let fee_payer = wallet
            .find_public_key(&self.settings.gas_payer.name)
            .map_err(|e| TaskError::Wallet(e.to_string()))?;
        let token_amount = token::Amount::from_u64(self.amount);
        let amount = InputAmount::Unvalidated(token::DenominatedAmount::native(token_amount));

        let source = TransferSource::Address(source_address.into_owned());
        let mut tx_builder = ctx.namada.new_ibc_transfer(
            source,
            self.receiver.name.clone(),
            token_address,
            amount,
            self.src_channel_id.clone(),
            false,
        );
        tx_builder = tx_builder.gas_limit(GasLimit::from(self.settings.gas_limit));
        tx_builder = tx_builder.wrapper_fee_payer(fee_payer);
        let mut signing_keys = vec![];
        for signer in &self.settings.signers {
            let public_key = wallet
                .find_public_key(&signer.name)
                .map_err(|e| TaskError::Wallet(e.to_string()))?;
            signing_keys.push(public_key)
        }
        tx_builder = tx_builder.signing_keys(signing_keys);
        drop(wallet);

        let (transfer_tx, signing_data, _) = tx_builder
            .build(&ctx.namada, &mut bparams)
            .await
            .map_err(|e| TaskError::BuildTx(e.to_string()))?;

        Ok((transfer_tx, vec![signing_data], tx_builder.tx))
    }

    async fn build_checks(
        &self,
        ctx: &Ctx,
        retry_config: RetryConfig,
    ) -> Result<Vec<Check>, TaskError> {
        let (_, pre_balance) = get_balance(ctx, &self.source, &self.denom, retry_config).await?;
        let source_check = Check::BalanceSource(
            check::balance_source::BalanceSource::builder()
                .target(self.source.clone())
                .pre_balance(pre_balance)
                .denom(self.denom.clone())
                .amount(self.amount)
                .build(),
        );

        let ibc_ack = Check::AckIbcTransfer(
            check::ack_ibc_transfer::AckIbcTransfer::builder()
                .source(self.source.clone())
                .receiver(self.receiver.clone())
                .src_channel_id(self.src_channel_id.clone())
                .dest_channel_id(self.dest_channel_id.clone())
                .build(),
        );

        Ok(vec![source_check, ibc_ack])
    }

    fn update_state(&self, state: &mut State) {
        if is_native_denom(&self.denom) {
            state.decrease_balance(&self.source, self.amount);
        } else {
            state.decrease_ibc_balance(&self.source, &self.denom, self.amount);
        }
        state.increase_foreign_balance(&self.receiver, self.amount);
    }
}

#[derive(Clone, Debug, TypedBuilder)]
pub struct IbcTransferRecv {
    sender: Alias,
    target: Alias,
    denom: String,
    amount: Amount,
    src_channel_id: ChannelId,
    dest_channel_id: ChannelId,
    settings: TaskSettings,
}

impl TaskContext for IbcTransferRecv {
    fn name(&self) -> String {
        "ibc-transfer-recv".to_string()
    }

    fn summary(&self) -> String {
        format!(
            "ibc-transfer-recv/{}/{}/'{}'/{}",
            self.sender.name, self.target.name, self.denom, self.amount
        )
    }

    fn task_settings(&self) -> Option<&TaskSettings> {
        Some(&self.settings)
    }

    async fn build_tx(&self, _ctx: &Ctx) -> Result<(Tx, Vec<SigningTxData>, args::Tx), TaskError> {
        unreachable!("Namada tx shouldn't be built")
    }

    async fn execute(&self, ctx: &Ctx) -> Result<Height, TaskError> {
        self.execute_cosmos_tx(ctx).await
    }

    async fn build_cosmos_tx(&self, ctx: &Ctx) -> Result<Any, TaskError> {
        let wallet = ctx.namada.wallet.read().await;
        let target_address = wallet
            .find_address(&self.target.name)
            .ok_or_else(|| TaskError::Wallet(format!("No source address: {}", self.target.name)))?
            .into_owned();

        let denom = if is_native_denom(&self.denom) {
            self.denom.clone()
        } else {
            let base_token = base_denom(&self.denom);
            let token_address = wallet.find_address(&base_token).ok_or_else(|| {
                TaskError::Wallet(format!("No native token address: {base_token}",))
            })?;
            let denom = self.denom.replace(&base_token, &token_address.to_string());
            cosmos_denom_hash(&denom)
        };
        drop(wallet);
        let any_msg = build_cosmos_ibc_transfer(
            &self.sender.name,
            &target_address.to_string(),
            &denom,
            self.amount,
            &PortId::transfer(),
            &self.src_channel_id,
            None,
        );

        Ok(any_msg)
    }

    async fn build_checks(
        &self,
        ctx: &Ctx,
        retry_config: RetryConfig,
    ) -> Result<Vec<Check>, TaskError> {
        let recv_packet = Check::RecvIbcPacket(
            check::recv_ibc_packet::RecvIbcPacket::builder()
                .sender(self.sender.clone())
                .target(self.target.clone())
                .src_channel_id(self.src_channel_id.clone())
                .dest_channel_id(self.dest_channel_id.clone())
                .build(),
        );

        let recv_denom = if is_native_denom(&self.denom) {
            ibc_denom(&self.dest_channel_id, &self.denom)
        } else {
            base_denom(&self.denom)
        };
        let (_, pre_balance) = get_balance(ctx, &self.target, &recv_denom, retry_config).await?;
        let target_check = Check::BalanceTarget(
            check::balance_target::BalanceTarget::builder()
                .target(self.target.clone())
                .pre_balance(pre_balance)
                .denom(recv_denom)
                .amount(self.amount)
                .build(),
        );

        // Check receiving IBC packet before checking the target balance
        Ok(vec![recv_packet, target_check])
    }

    fn update_state(&self, state: &mut State) {
        if self.denom == ibc_denom(&self.dest_channel_id, &Alias::nam().name) {
            state.increase_balance(&self.target, self.amount);
        } else {
            let ibc_denom = ibc_denom(&self.dest_channel_id, &self.denom);
            state.increase_ibc_balance(&self.target, &ibc_denom, self.amount);
        }
    }
}
