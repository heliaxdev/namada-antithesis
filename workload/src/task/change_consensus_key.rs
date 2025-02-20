use namada_sdk::args::{self, TxBuilder};
use namada_sdk::key::{RefTo, SchemeType};
use namada_sdk::signing::SigningTxData;
use namada_sdk::tx::data::GasLimit;
use namada_sdk::tx::Tx;
use namada_sdk::Namada;
use rand::rngs::OsRng;
use typed_builder::TypedBuilder;

use crate::check::Check;
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::task::{TaskContext, TaskSettings};
use crate::types::Alias;
use crate::utils::RetryConfig;

#[derive(Clone, TypedBuilder)]
pub struct ChangeConsensusKey {
    source: Alias,
    consensus_alias: Alias,
    settings: TaskSettings,
}

impl TaskContext for ChangeConsensusKey {
    fn name(&self) -> String {
        "change-consensus-key".to_string()
    }

    fn summary(&self) -> String {
        format!("change-consensus-key/{}", self.source.name)
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

        let source_address = wallet
            .find_address(&self.source.name)
            .ok_or_else(|| StepError::Wallet(format!("No source address: {}", self.source.name)))?;
        let fee_payer = wallet
            .find_public_key(&self.settings.gas_payer.name)
            .map_err(|e| StepError::Wallet(e.to_string()))?;

        let mut change_consensus_key_builder = sdk
            .namada
            .new_change_consensus_key(source_address.into_owned(), consensus_pk);

        change_consensus_key_builder =
            change_consensus_key_builder.gas_limit(GasLimit::from(self.settings.gas_limit));
        change_consensus_key_builder = change_consensus_key_builder.wrapper_fee_payer(fee_payer);

        let mut signing_keys = vec![];
        for signer in &self.settings.signers {
            let public_key = wallet
                .find_public_key(&signer.name)
                .map_err(|e| StepError::Wallet(e.to_string()))?;
            signing_keys.push(public_key)
        }
        change_consensus_key_builder = change_consensus_key_builder.signing_keys(signing_keys);
        drop(wallet);

        let (change_consensus_key, signing_data) = change_consensus_key_builder
            .build(&sdk.namada)
            .await
            .map_err(|e| StepError::Build(e.to_string()))?;

        Ok((
            change_consensus_key,
            vec![signing_data],
            change_consensus_key_builder.tx,
        ))
    }

    async fn build_checks(
        &self,
        _sdk: &Sdk,
        _retry_config: RetryConfig,
    ) -> Result<Vec<Check>, StepError> {
        Ok(vec![])
    }

    fn update_state(&self, state: &mut State, with_fee: bool) {
        if with_fee {
            state.modify_balance_fee(&self.settings.gas_payer, self.settings.gas_limit);
        }
    }
}
