use namada_sdk::{
    args::{self, InputAmount, TxBuilder, TxTransparentTransferData},
    signing::SigningTxData,
    token::{self, DenominatedAmount},
    tx::{data::GasLimit, Tx},
    Namada,
};

use crate::{
    check::Check, entities::Alias, executor::StepError, sdk::namada::Sdk, task::TaskSettings,
};

use super::query_utils::get_balance;
use super::{RetryConfig, TaskContext};

#[derive(Clone, Debug)]
pub(super) struct FaucetTransfer {
    target: Alias,
    amount: u64,
    settings: TaskSettings,
}

impl TaskContext for FaucetTransfer {
    async fn build_tx(&self, sdk: &Sdk) -> Result<(Tx, Vec<SigningTxData>, args::Tx), StepError> {
        let wallet = sdk.namada.wallet.read().await;

        let faucet_alias = Alias::faucet();
        let native_token_alias = Alias::nam();

        let source_address = wallet.find_address(&faucet_alias.name).ok_or_else(|| {
            StepError::Wallet(format!("No source address: {}", faucet_alias.name))
        })?;
        let target_address = wallet
            .find_address(&self.target.name)
            .ok_or_else(|| StepError::Wallet(format!("No target address: {}", self.target.name)))?;
        let token_address = wallet
            .find_address(&native_token_alias.name)
            .ok_or_else(|| {
                StepError::Wallet(format!(
                    "No native token address: {}",
                    native_token_alias.name
                ))
            })?;
        let fee_payer = wallet
            .find_public_key(&self.settings.gas_payer.name)
            .map_err(|e| StepError::Wallet(e.to_string()))?;
        let token_amount = token::Amount::from_u64(self.amount);

        let tx_transfer_data = TxTransparentTransferData {
            source: source_address.into_owned(),
            target: target_address.into_owned(),
            token: token_address.into_owned(),
            amount: InputAmount::Unvalidated(DenominatedAmount::native(token_amount)),
        };

        let mut transfer_tx_builder = sdk.namada.new_transparent_transfer(vec![tx_transfer_data]);

        transfer_tx_builder =
            transfer_tx_builder.gas_limit(GasLimit::from(self.settings.gas_limit));
        transfer_tx_builder = transfer_tx_builder.wrapper_fee_payer(fee_payer);

        let mut signing_keys = vec![];
        for signer in &self.settings.signers {
            let public_key = wallet
                .find_public_key(&signer.name)
                .map_err(|e| StepError::Wallet(e.to_string()))?;
            signing_keys.push(public_key)
        }
        transfer_tx_builder = transfer_tx_builder.signing_keys(signing_keys);
        drop(wallet);

        let (transfer_tx, signing_data) = transfer_tx_builder
            .build(&sdk.namada)
            .await
            .map_err(|e| StepError::Build(e.to_string()))?;

        Ok((transfer_tx, vec![signing_data], transfer_tx_builder.tx))
    }

    async fn build_checks(
        &self,
        sdk: &Sdk,
        retry_config: RetryConfig,
    ) -> Result<Vec<Check>, StepError> {
        let pre_balance = get_balance(sdk, &self.target, retry_config).await?;

        Ok(vec![Check::BalanceTarget(
            self.target.clone(),
            pre_balance,
            self.amount,
        )])
    }
}
