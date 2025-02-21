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
pub struct Redelegate {
    source: Alias,
    from_validator: ValidatorAddress,
    to_validator: ValidatorAddress,
    amount: Amount,
    epoch: Epoch,
    settings: TaskSettings,
}

impl TaskContext for Redelegate {
    fn name(&self) -> String {
        "redelegate".to_string()
    }

    fn summary(&self) -> String {
        format!(
            "redelegate/{}/{}/{}/{}",
            self.source.name, self.from_validator, self.to_validator, self.amount
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
        let fee_payer = wallet
            .find_public_key(&self.settings.gas_payer.name)
            .map_err(|e| StepError::Wallet(e.to_string()))?;
        let from_validator =
            Address::from_str(&self.from_validator).expect("ValidatorAddress should be converted");
        let to_validator =
            Address::from_str(&self.to_validator).expect("ValidatorAddress should be converted");

        let mut redelegate_tx_builder = sdk.namada.new_redelegation(
            source_address.into_owned(),
            from_validator,
            to_validator,
            token_amount,
        );
        redelegate_tx_builder =
            redelegate_tx_builder.gas_limit(GasLimit::from(self.settings.gas_limit));
        redelegate_tx_builder = redelegate_tx_builder.wrapper_fee_payer(fee_payer);
        let mut signing_keys = vec![];
        for signer in &self.settings.signers {
            let public_key = wallet
                .find_public_key(&signer.name)
                .map_err(|e| StepError::Wallet(e.to_string()))?;
            signing_keys.push(public_key)
        }
        redelegate_tx_builder = redelegate_tx_builder.signing_keys(signing_keys);
        drop(wallet);

        let (bond_tx, signing_data) = redelegate_tx_builder
            .build(&sdk.namada)
            .await
            .map_err(|e| StepError::BuildTx(e.to_string()))?;

        Ok((bond_tx, vec![signing_data], redelegate_tx_builder.tx))
    }

    async fn build_checks(
        &self,
        sdk: &Sdk,
        retry_config: RetryConfig,
    ) -> Result<Vec<Check>, StepError> {
        let pre_bond = get_bond(
            sdk,
            &self.source,
            &self.from_validator,
            self.epoch,
            retry_config,
        )
        .await?;
        let from_validator_bond_check = Check::BondDecrease(
            check::bond_decrease::BondDecrease::builder()
                .target(self.source.clone())
                .validator(self.from_validator.clone())
                .pre_bond(pre_bond)
                .epoch(self.epoch)
                .amount(self.amount)
                .build(),
        );

        let pre_bond = get_bond(
            sdk,
            &self.source,
            &self.to_validator,
            self.epoch,
            retry_config,
        )
        .await?;

        let to_validator_bond_check = Check::BondIncrease(
            check::bond_increase::BondIncrease::builder()
                .target(self.source.clone())
                .validator(self.to_validator.clone())
                .pre_bond(pre_bond)
                .epoch(self.epoch)
                .amount(self.amount)
                .build(),
        );

        Ok(vec![from_validator_bond_check, to_validator_bond_check])
    }

    fn update_state(&self, state: &mut State, with_fee: bool) {
        if with_fee {
            state.modify_balance_fee(&self.settings.gas_payer, self.settings.gas_limit);
        }
        state.modify_redelegate(
            &self.source,
            &self.from_validator,
            &self.to_validator,
            self.amount,
        )
    }
}
