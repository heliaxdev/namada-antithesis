use std::str::FromStr;

use namada_sdk::address::Address;
use namada_sdk::args::{self, TxBuilder};
use namada_sdk::signing::SigningTxData;
use namada_sdk::token;
use namada_sdk::tx::data::GasLimit;
use namada_sdk::tx::Tx;
use namada_sdk::Namada;
use typed_builder::TypedBuilder;

use crate::check::{self, Check};
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::task::{TaskContext, TaskSettings};
use crate::types::{Alias, Amount, Epoch, ValidatorAddress};
use crate::utils::{get_bond, RetryConfig};

#[derive(Clone, Debug, TypedBuilder)]
pub struct Unbond {
    source: Alias,
    validator: ValidatorAddress,
    amount: Amount,
    epoch: Epoch,
    settings: TaskSettings,
}

impl TaskContext for Unbond {
    fn name(&self) -> String {
        "unbond".to_string()
    }

    fn summary(&self) -> String {
        format!(
            "unbond/{}/{}/{}",
            self.source.name, self.validator, self.amount
        )
    }

    fn task_settings(&self) -> Option<&TaskSettings> {
        Some(&self.settings)
    }

    async fn build_tx(&self, sdk: &Sdk) -> Result<(Tx, Vec<SigningTxData>, args::Tx), StepError> {
        let wallet = sdk.namada.wallet.read().await;

        let source_address = wallet
            .find_address(&self.source.name)
            .ok_or_else(|| StepError::Wallet(format!("No source address: {}", self.source.name)))?;
        let token_amount = token::Amount::from_u64(self.amount);
        let validator =
            Address::from_str(&self.validator).expect("ValidatorAddress should be converted");

        let mut unbond_tx_builder = sdk
            .namada
            .new_unbond(validator, token_amount)
            .source(source_address.into_owned());
        unbond_tx_builder = unbond_tx_builder.gas_limit(GasLimit::from(self.settings.gas_limit));
        let mut signing_keys = vec![];
        for signer in &self.settings.signers {
            let public_key = wallet
                .find_public_key(&signer.name)
                .map_err(|e| StepError::Wallet(e.to_string()))?;
            signing_keys.push(public_key)
        }
        unbond_tx_builder = unbond_tx_builder.signing_keys(signing_keys);
        drop(wallet);

        let (unbond_tx, signing_data, _epoch) = unbond_tx_builder
            .build(&sdk.namada)
            .await
            .map_err(|e| StepError::BuildTx(e.to_string()))?;

        Ok((unbond_tx, vec![signing_data], unbond_tx_builder.tx))
    }

    async fn build_checks(
        &self,
        sdk: &Sdk,
        retry_config: RetryConfig,
    ) -> Result<Vec<Check>, StepError> {
        let pre_bond =
            get_bond(sdk, &self.source, &self.validator, self.epoch, retry_config).await?;

        Ok(vec![Check::BondDecrease(
            check::bond_decrease::BondDecrease::builder()
                .target(self.source.clone())
                .validator(self.validator.clone())
                .pre_bond(pre_bond)
                .epoch(self.epoch)
                .amount(self.amount)
                .build(),
        )])
    }

    fn update_state(&self, state: &mut State) {
        state.modify_unbond(&self.source, &self.validator, self.amount);
    }
}
