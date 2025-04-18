use namada_sdk::args::{self, InputAmount, TxBuilder, TxTransparentTransferData};
use namada_sdk::signing::SigningTxData;
use namada_sdk::token::{self, DenominatedAmount};
use namada_sdk::tx::data::GasLimit;
use namada_sdk::tx::Tx;
use namada_sdk::Namada;
use typed_builder::TypedBuilder;

use crate::check::{self, Check};
use crate::context::Ctx;
use crate::error::TaskError;
use crate::state::State;
use crate::task::{TaskContext, TaskSettings};
use crate::types::{Alias, Amount};
use crate::utils::{get_balance, RetryConfig};

#[derive(Clone, Debug, TypedBuilder)]
pub struct FaucetTransfer {
    target: Alias,
    amount: Amount,
    settings: TaskSettings,
}

impl TaskContext for FaucetTransfer {
    fn name(&self) -> String {
        "faucet-transfer".to_string()
    }

    fn summary(&self) -> String {
        format!("faucet-transfer/{}/{}", self.target.name, self.amount)
    }

    fn task_settings(&self) -> Option<&TaskSettings> {
        Some(&self.settings)
    }

    async fn build_tx(&self, ctx: &Ctx) -> Result<(Tx, Vec<SigningTxData>, args::Tx), TaskError> {
        let wallet = ctx.namada.wallet.read().await;

        let faucet_alias = Alias::faucet();
        let native_token_alias = Alias::nam();

        let source_address = wallet.find_address(&faucet_alias.name).ok_or_else(|| {
            TaskError::Wallet(format!("No source address: {}", faucet_alias.name))
        })?;
        let target_address = wallet
            .find_address(&self.target.name)
            .ok_or_else(|| TaskError::Wallet(format!("No target address: {}", self.target.name)))?;
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

        let tx_transfer_data = TxTransparentTransferData {
            source: source_address.into_owned(),
            target: target_address.into_owned(),
            token: token_address.into_owned(),
            amount: InputAmount::Unvalidated(DenominatedAmount::native(token_amount)),
        };

        let mut transfer_tx_builder = ctx.namada.new_transparent_transfer(vec![tx_transfer_data]);

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

        let (transfer_tx, signing_data) = transfer_tx_builder
            .build(&ctx.namada)
            .await
            .map_err(|e| TaskError::BuildTx(e.to_string()))?;

        Ok((transfer_tx, vec![signing_data], transfer_tx_builder.tx))
    }

    async fn build_checks(
        &self,
        ctx: &Ctx,
        retry_config: RetryConfig,
    ) -> Result<Vec<Check>, TaskError> {
        let denom = Alias::nam().name;
        let (_, pre_balance) = get_balance(ctx, &self.target, &denom, retry_config).await?;

        Ok(vec![Check::BalanceTarget(
            check::balance_target::BalanceTarget::builder()
                .target(self.target.clone())
                .pre_balance(pre_balance)
                .denom(denom)
                .amount(self.amount)
                .build(),
        )])
    }

    fn update_state(&self, state: &mut State) {
        state.increase_balance(&self.target, self.amount);
    }
}
