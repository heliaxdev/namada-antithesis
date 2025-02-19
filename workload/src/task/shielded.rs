use namada_sdk::{
    args::{self, InputAmount, TxBuilder, TxShieldedTransferData},
    masp_primitives::{
        self, transaction::components::sapling::builder::RngBuildParams, zip32::PseudoExtendedKey,
    },
    signing::SigningTxData,
    token,
    tx::{data::GasLimit, Tx},
    Namada,
};
use rand::rngs::OsRng;

use crate::{
    check::Check,
    entities::Alias,
    executor::StepError,
    sdk::namada::Sdk,
    task::{Amount, TaskSettings},
};

use super::query_utils::get_shielded_balance;
use super::{RetryConfig, TaskContext};

#[derive(Clone, Debug)]
pub(super) struct ShieldedTransfer {
    source: Alias,
    target: Alias,
    amount: Amount,
    settings: TaskSettings,
}

impl TaskContext for ShieldedTransfer {
    async fn build_tx(&self, sdk: &Sdk) -> Result<(Tx, Vec<SigningTxData>, args::Tx), StepError> {
        let mut bparams = RngBuildParams::new(OsRng);
        let mut wallet = sdk.namada.wallet.write().await;

        let native_token_alias = Alias::nam();

        let source_spending_key = wallet
            .find_spending_key(&self.source.name, None)
            .map_err(|e| StepError::Wallet(e.to_string()))?;
        let tmp = masp_primitives::zip32::ExtendedSpendingKey::from(source_spending_key);
        let pseudo_spending_key_from_spending_key = PseudoExtendedKey::from(tmp);
        let target_payment_address =
            *wallet.find_payment_addr(&self.target.name).ok_or_else(|| {
                StepError::Wallet(format!("No payment address: {}", self.target.name))
            })?;
        let token = wallet
            .find_address(&native_token_alias.name)
            .ok_or_else(|| {
                StepError::Wallet(format!(
                    "No native token address: {}",
                    native_token_alias.name
                ))
            })?;
        let token_amount = token::Amount::from_u64(self.amount);
        let amount = InputAmount::Unvalidated(token::DenominatedAmount::native(token_amount));
        let fee_payer = wallet
            .find_public_key(&self.settings.gas_payer.name)
            .map_err(|e| StepError::Wallet(e.to_string()))?;
        let tx_transfer_data = TxShieldedTransferData {
            source: pseudo_spending_key_from_spending_key,
            target: target_payment_address,
            token: token.into_owned(),
            amount,
        };

        // FIXME review the gaspayer
        let mut transfer_tx_builder =
            sdk.namada
                .new_shielded_transfer(vec![tx_transfer_data], None, false);
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
            .build(&sdk.namada, &mut bparams)
            .await
            .map_err(|e| StepError::Build(e.to_string()))?;

        Ok((transfer_tx, vec![signing_data], transfer_tx_builder.tx))
    }

    async fn build_checks(
        &self,
        sdk: &Sdk,
        retry_config: RetryConfig,
    ) -> Result<Vec<Check>, StepError> {
        let pre_balance = get_shielded_balance(sdk, &self.source, None, false)
            .await?
            .unwrap_or_default();
        let source_check =
            Check::BalanceShieldedSource(self.source.clone(), pre_balance, self.amount);

        let pre_balance = get_shielded_balance(sdk, &self.target, None, false)
            .await?
            .unwrap_or_default();
        let target_check =
            Check::BalanceShieldedTarget(self.target.clone(), pre_balance, self.amount);

        Ok(vec![source_check, target_check])
    }
}
