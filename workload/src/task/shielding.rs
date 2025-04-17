use namada_sdk::args::{self, InputAmount, TxBuilder, TxShieldingTransferData};
use namada_sdk::masp_primitives::transaction::components::sapling::builder::RngBuildParams;
use namada_sdk::signing::SigningTxData;
use namada_sdk::token::{self, DenominatedAmount};
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
use crate::utils::{get_balance, get_shielded_balance, shielded_sync_with_retry, RetryConfig};

#[derive(Clone, Debug, TypedBuilder)]
pub struct Shielding {
    source: Alias,
    target: Alias,
    amount: Amount,
    epoch: MaspEpoch,
    settings: TaskSettings,
}

impl Shielding {
    pub fn epoch(&self) -> MaspEpoch {
        self.epoch
    }
}

impl TaskContext for Shielding {
    fn name(&self) -> String {
        "shielding".to_string()
    }

    fn summary(&self) -> String {
        format!(
            "shielding/{}/{}/{}",
            self.source.name, self.target.name, self.amount
        )
    }

    fn task_settings(&self) -> Option<&TaskSettings> {
        Some(&self.settings)
    }

    async fn build_tx(&self, ctx: &Ctx) -> Result<(Tx, Vec<SigningTxData>, args::Tx), TaskError> {
        let mut bparams = RngBuildParams::new(OsRng);

        let wallet = ctx.namada.wallet.read().await;

        let native_token_alias = Alias::nam();

        let source_address = wallet
            .find_address(&self.source.name)
            .ok_or_else(|| TaskError::Wallet(format!("No source address: {}", self.source.name)))?;
        let target_payment_address =
            *wallet.find_payment_addr(&self.target.name).ok_or_else(|| {
                TaskError::Wallet(format!("No payment address: {}", self.target.name))
            })?;
        let token_address = wallet
            .find_address(&native_token_alias.name)
            .ok_or_else(|| {
                TaskError::Wallet(format!(
                    "No native token address: {}",
                    native_token_alias.name
                ))
            })?;
        let fee_payer = wallet
            .find_public_key(&self.settings.gas_payer.name)
            .map_err(|e| TaskError::Wallet(e.to_string()))?;
        let token_amount = token::Amount::from_u64(self.amount);

        let tx_transfer_data = TxShieldingTransferData {
            source: source_address.into_owned(),
            token: token_address.into_owned(),
            amount: InputAmount::Unvalidated(DenominatedAmount::native(token_amount)),
        };

        let mut transfer_tx_builder = ctx
            .namada
            .new_shielding_transfer(target_payment_address, vec![tx_transfer_data]);
        transfer_tx_builder =
            transfer_tx_builder.gas_limit(GasLimit::from(self.settings.gas_limit));
        transfer_tx_builder = transfer_tx_builder.wrapper_fee_payer(fee_payer);
        let mut signing_keys = vec![];
        for signer in &self.settings.signers {
            let public_key = wallet
                .find_public_key(&signer.name)
                .map_err(|e| TaskError::Wallet(e.to_string()))?;
            signing_keys.push(public_key)
        }
        transfer_tx_builder = transfer_tx_builder.signing_keys(signing_keys);
        drop(wallet);

        let (transfer_tx, signing_data, _epoch) = transfer_tx_builder
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
        let denom = Alias::nam().name;
        let (_, pre_balance) = get_balance(ctx, &self.source, &denom, retry_config).await?;
        let source_check = Check::BalanceSource(
            check::balance_source::BalanceSource::builder()
                .target(self.source.clone())
                .pre_balance(pre_balance)
                .denom(denom.clone())
                .amount(self.amount)
                .build(),
        );

        shielded_sync_with_retry(ctx, &self.target, None, false, retry_config).await?;

        let pre_balance = get_shielded_balance(ctx, &self.target, &denom, retry_config)
            .await?
            .unwrap_or_default();
        let target_check = Check::BalanceShieldedTarget(
            check::balance_shielded_target::BalanceShieldedTarget::builder()
                .target(self.target.clone())
                .pre_balance(pre_balance)
                .denom(denom)
                .amount(self.amount)
                .build(),
        );

        Ok(vec![source_check, target_check])
    }

    fn update_state(&self, state: &mut State) {
        state.modify_shielding(&self.source, &self.target, self.amount)
    }
}
