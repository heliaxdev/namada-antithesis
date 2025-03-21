use namada_sdk::args::{self, InputAmount, TxBuilder, TxUnshieldingTransferData};
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
use crate::error::TaskError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::task::{TaskContext, TaskSettings};
use crate::types::{Alias, Amount, Height};
use crate::utils::{
    execute_shielded_tx, get_balance, get_epoch, get_shielded_balance, retry_config,
    shielded_sync_with_retry, RetryConfig,
};

#[derive(Clone, Debug, TypedBuilder)]
pub struct Unshielding {
    source: Alias,
    target: Alias,
    amount: Amount,
    settings: TaskSettings,
}

impl Unshielding {
    pub fn source(&self) -> &Alias {
        &self.source
    }
}

impl TaskContext for Unshielding {
    fn name(&self) -> String {
        "unshielding".to_string()
    }

    fn summary(&self) -> String {
        format!(
            "unshielding/{}/{}/{}",
            self.source.name, self.target.name, self.amount
        )
    }

    fn task_settings(&self) -> Option<&TaskSettings> {
        Some(&self.settings)
    }

    async fn build_tx(&self, sdk: &Sdk) -> Result<(Tx, Vec<SigningTxData>, args::Tx), TaskError> {
        let mut bparams = RngBuildParams::new(OsRng);

        let mut wallet = sdk.namada.wallet.write().await;

        let source_spending_key = wallet
            .find_spending_key(&self.source.name, None)
            .map_err(|e| TaskError::Wallet(e.to_string()))?;
        let tmp = masp_primitives::zip32::ExtendedSpendingKey::from(source_spending_key);
        let pseudo_spending_key_from_spending_key = PseudoExtendedKey::from(tmp);
        let target_address = wallet
            .find_address(&self.target.name)
            .ok_or_else(|| TaskError::Wallet(format!("No target address: {}", self.target.name)))?;

        let native_token_alias = Alias::nam();
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

        let tx_transfer_data = TxUnshieldingTransferData {
            target: target_address.into_owned(),
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

        let mut transfer_tx_builder = sdk.namada.new_unshielding_transfer(
            pseudo_spending_key_from_spending_key,
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

        // signing key isn't needed for unshielding transfer

        let (transfer_tx, signing_data) = transfer_tx_builder
            .build(&sdk.namada, &mut bparams)
            .await
            .map_err(|e| TaskError::BuildTx(e.to_string()))?;

        Ok((transfer_tx, vec![signing_data], transfer_tx_builder.tx))
    }

    async fn execute(&self, sdk: &Sdk) -> Result<Height, TaskError> {
        let start_epoch = get_epoch(sdk, retry_config()).await?;
        let (tx, signing_data, tx_args) = self.build_tx(sdk).await?;
        execute_shielded_tx(sdk, tx, signing_data, &tx_args, start_epoch).await
    }

    async fn build_checks(
        &self,
        sdk: &Sdk,
        retry_config: RetryConfig,
    ) -> Result<Vec<Check>, TaskError> {
        shielded_sync_with_retry(sdk, &self.source, None, false).await?;

        let pre_balance = get_shielded_balance(sdk, &self.source, retry_config)
            .await?
            .unwrap_or_default();
        let source_check = Check::BalanceShieldedSource(
            check::balance_shielded_source::BalanceShieldedSource::builder()
                .target(self.source.clone())
                .pre_balance(pre_balance)
                .amount(self.amount)
                .build(),
        );

        let (_, pre_balance) = get_balance(sdk, &self.target, retry_config).await?;
        let target_check = Check::BalanceTarget(
            check::balance_target::BalanceTarget::builder()
                .target(self.target.clone())
                .pre_balance(pre_balance)
                .amount(self.amount)
                .build(),
        );

        Ok(vec![source_check, target_check])
    }

    fn update_state(&self, state: &mut State) {
        state.modify_unshielding(&self.source, &self.target, self.amount)
    }
}
