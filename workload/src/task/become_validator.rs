use namada_sdk::key::RefTo;
use namada_sdk::{
    args::{self, TxBuilder},
    key::SchemeType,
    signing::SigningTxData,
    tx::{data::GasLimit, Tx},
    Namada,
};
use rand::rngs::OsRng;
use typed_builder::TypedBuilder;

use crate::check::Check;
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::task::{TaskContext, TaskSettings};
use crate::types::{Alias, CommissionChange, CommissionRate};
use crate::utils::RetryConfig;

#[derive(Clone, TypedBuilder)]
pub struct BecomeValidator {
    source: Alias,
    consensus_alias: Alias,
    eth_cold_alias: Alias,
    eth_hot_alias: Alias,
    protocol_alias: Alias,
    commission_rate: CommissionRate,
    commission_max_change: CommissionChange,
    settings: TaskSettings,
}

impl TaskContext for BecomeValidator {
    fn name(&self) -> String {
        "become-validator".to_string()
    }

    fn summary(&self) -> String {
        format!("become-validator/{}", self.source.name)
    }

    fn task_settings(&self) -> Option<&TaskSettings> {
        Some(&self.settings)
    }

    async fn build_tx(&self, sdk: &Sdk) -> Result<(Tx, Vec<SigningTxData>, args::Tx), StepError> {
        let mut wallet = sdk.namada.wallet.write().await;

        let consensus_pk = wallet
            .gen_store_secret_key(
                SchemeType::Ed25519,
                Some(self.consensus_alias.name.clone()),
                true,
                None,
                &mut OsRng,
            )
            .expect("Key generation should not fail.")
            .1
            .ref_to();

        let eth_cold_pk = wallet
            .gen_store_secret_key(
                SchemeType::Secp256k1,
                Some(self.eth_cold_alias.name.clone()),
                true,
                None,
                &mut OsRng,
            )
            .expect("Key generation should not fail.")
            .1
            .ref_to();

        let eth_hot_pk = wallet
            .gen_store_secret_key(
                SchemeType::Secp256k1,
                Some(self.eth_hot_alias.name.clone()),
                true,
                None,
                &mut OsRng,
            )
            .expect("Key generation should not fail.")
            .1
            .ref_to();

        let protocol_key = wallet
            .gen_store_secret_key(
                SchemeType::Ed25519,
                Some(self.protocol_alias.name.clone()),
                true,
                None,
                &mut OsRng,
            )
            .expect("Key generation should not fail.")
            .1
            .ref_to();

        let source_address = wallet
            .find_address(&self.source.name)
            .ok_or_else(|| StepError::Wallet(format!("No source address: {}", self.source.name)))?;
        let fee_payer = wallet
            .find_public_key(&self.settings.gas_payer.name)
            .map_err(|e| StepError::Wallet(e.to_string()))?;
        wallet
            .save()
            .map_err(|e| StepError::Wallet(format!("Failed to save the wallet: {e}")))?;

        let mut become_validator_tx_builder = sdk
            .namada
            .new_become_validator(
                source_address.into_owned(),
                self.commission_rate,
                self.commission_max_change,
                consensus_pk,
                eth_cold_pk,
                eth_hot_pk,
                protocol_key,
                "test@test.it".to_string(),
            )
            .wallet_alias_force(true);

        become_validator_tx_builder =
            become_validator_tx_builder.gas_limit(GasLimit::from(self.settings.gas_limit));
        become_validator_tx_builder = become_validator_tx_builder.wrapper_fee_payer(fee_payer);

        let mut signing_keys = vec![];
        for signer in &self.settings.signers {
            let public_key = wallet
                .find_public_key(&signer.name)
                .map_err(|e| StepError::Wallet(e.to_string()))?;
            signing_keys.push(public_key)
        }
        become_validator_tx_builder = become_validator_tx_builder.signing_keys(signing_keys);
        drop(wallet);

        let (become_validator, signing_data) = become_validator_tx_builder
            .build(&sdk.namada)
            .await
            .map_err(|e| StepError::Build(e.to_string()))?;

        Ok((
            become_validator,
            vec![signing_data],
            become_validator_tx_builder.tx,
        ))
    }

    async fn build_checks(
        &self,
        _sdk: &Sdk,
        _retry_config: RetryConfig,
    ) -> Result<Vec<Check>, StepError> {
        Ok(vec![Check::IsValidatorAccount(self.source.clone())])
    }

    fn update_state(&self, state: &mut State, with_fee: bool) {
        if with_fee {
            state.modify_balance_fee(&self.settings.gas_payer, self.settings.gas_limit);
        }
        state.set_enstablished_as_validator(&self.source)
    }
}
