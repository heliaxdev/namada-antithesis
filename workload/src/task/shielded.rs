use namada_sdk::args::{self, InputAmount, TxBuilder, TxShieldedTransferData};
use namada_sdk::masp_primitives;
use namada_sdk::masp_primitives::transaction::components::sapling::builder::RngBuildParams;
use namada_sdk::masp_primitives::zip32::PseudoExtendedKey;
use namada_sdk::signing::SigningTxData;
use namada_sdk::token;
use namada_sdk::tx::data::GasLimit;
use namada_sdk::tx::Tx;
use namada_sdk::Namada;
use rand::rngs::OsRng;
use typed_builder::TypedBuilder;

use crate::check::{self, Check};
use crate::context::Ctx;
use crate::error::TaskError;
use crate::state::State;
use crate::task::{TaskContext, TaskSettings};
use crate::types::{Alias, Amount, Height, MaspEpoch};
use crate::utils::{get_shielded_balance, shielded_sync_with_retry, RetryConfig};

#[derive(Clone, Debug, TypedBuilder)]
pub struct ShieldedTransfer {
    source: Alias,
    target: Alias,
    amount: Amount,
    epoch: MaspEpoch,
    settings: TaskSettings,
}

impl ShieldedTransfer {
    pub fn epoch(&self) -> MaspEpoch {
        self.epoch
    }
}

impl ShieldedTransfer {
    pub fn source(&self) -> &Alias {
        &self.source
    }
}

impl TaskContext for ShieldedTransfer {
    fn name(&self) -> String {
        "shielded-transfer".to_string()
    }

    fn summary(&self) -> String {
        format!(
            "shielded-transfer/{}/{}/{}",
            self.source.name, self.target.name, self.amount
        )
    }

    fn task_settings(&self) -> Option<&TaskSettings> {
        Some(&self.settings)
    }

    async fn build_tx(&self, ctx: &Ctx) -> Result<(Tx, Vec<SigningTxData>, args::Tx), TaskError> {
        let mut bparams = RngBuildParams::new(OsRng);
        let mut wallet = ctx.namada.wallet.write().await;

        let native_token_alias = Alias::nam();

        let source_spending_key = wallet
            .find_spending_key(&self.source.name, None)
            .map_err(|e| TaskError::Wallet(e.to_string()))?;
        let tmp = masp_primitives::zip32::ExtendedSpendingKey::from(source_spending_key);
        let pseudo_spending_key_from_spending_key = PseudoExtendedKey::from(tmp);
        let target_payment_address =
            *wallet.find_payment_addr(&self.target.name).ok_or_else(|| {
                TaskError::Wallet(format!("No payment address: {}", self.target.name))
            })?;
        let token = wallet
            .find_address(&native_token_alias.name)
            .ok_or_else(|| {
                TaskError::Wallet(format!(
                    "No native token address: {}",
                    native_token_alias.name
                ))
            })?;
        let token_amount = token::Amount::from_u64(self.amount);
        let amount = InputAmount::Unvalidated(token::DenominatedAmount::native(token_amount));
        let tx_transfer_data = TxShieldedTransferData {
            source: pseudo_spending_key_from_spending_key,
            target: target_payment_address,
            token: token.into_owned(),
            amount,
        };

        let disposable_gas_payer = self.settings.gas_payer.is_spending_key();
        let gas_spending_key = if disposable_gas_payer {
            let spending_key = wallet
                .find_spending_key(&self.settings.gas_payer.name, None)
                .map_err(|e| TaskError::Wallet(e.to_string()))?;
            let tmp = masp_primitives::zip32::ExtendedSpendingKey::from(spending_key);
            Some(PseudoExtendedKey::from(tmp))
        } else {
            None
        };

        let mut transfer_tx_builder = ctx.namada.new_shielded_transfer(
            vec![tx_transfer_data],
            gas_spending_key,
            disposable_gas_payer,
        );
        transfer_tx_builder =
            transfer_tx_builder.gas_limit(GasLimit::from(self.settings.gas_limit));
        if !disposable_gas_payer {
            let fee_payer = wallet
                .find_public_key(&self.settings.gas_payer.name)
                .map_err(|e| TaskError::Wallet(e.to_string()))?;
            transfer_tx_builder = transfer_tx_builder.wrapper_fee_payer(fee_payer);
        }
        drop(wallet);

        // signing key isn't needed for shielded transfer

        let (transfer_tx, signing_data) = transfer_tx_builder
            .build(&ctx.namada, &mut bparams)
            .await
            .map_err(|e| TaskError::BuildTx(e.to_string()))?;

        Ok((transfer_tx, vec![signing_data], transfer_tx_builder.tx))
    }

    async fn execute(&self, ctx: &Ctx) -> Result<Height, TaskError> {
        self.execute_shielded_tx(ctx, self.epoch).await
    }

    async fn build_checks(
        &self,
        ctx: &Ctx,
        retry_config: RetryConfig,
    ) -> Result<Vec<Check>, TaskError> {
        shielded_sync_with_retry(ctx, &self.source, None, false, retry_config).await?;

        let pre_balance = get_shielded_balance(ctx, &self.source, retry_config)
            .await?
            .unwrap_or_default();
        let source_check = Check::BalanceShieldedSource(
            check::balance_shielded_source::BalanceShieldedSource::builder()
                .target(self.source.clone())
                .pre_balance(pre_balance)
                .amount(self.amount)
                .build(),
        );

        let pre_balance = get_shielded_balance(ctx, &self.target, retry_config)
            .await?
            .unwrap_or_default();
        let target_check = Check::BalanceShieldedTarget(
            check::balance_shielded_target::BalanceShieldedTarget::builder()
                .target(self.target.clone())
                .pre_balance(pre_balance)
                .amount(self.amount)
                .build(),
        );

        Ok(vec![source_check, target_check])
    }

    fn update_state(&self, state: &mut State) {
        state.modify_shielded_transfer(&self.source, &self.target, self.amount);
    }
}
