use namada_sdk::args::{self, TxBuilder};
use namada_sdk::signing::SigningTxData;
use namada_sdk::tx::data::GasLimit;
use namada_sdk::tx::Tx;
use namada_sdk::Namada;
use typed_builder::TypedBuilder;

use crate::check::{self, Check};
use crate::error::TaskError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::task::{TaskContext, TaskSettings};
use crate::types::{Alias, ValidatorStatus};
use crate::utils::RetryConfig;

#[derive(Clone, Debug, TypedBuilder)]
pub struct DeactivateValidator {
    target: Alias,
    settings: TaskSettings,
}

impl TaskContext for DeactivateValidator {
    fn name(&self) -> String {
        "deactivate-validator".to_string()
    }

    fn summary(&self) -> String {
        format!("deactivate-validator/{}", self.target.name)
    }

    fn task_settings(&self) -> Option<&TaskSettings> {
        Some(&self.settings)
    }

    async fn build_tx(&self, sdk: &Sdk) -> Result<(Tx, Vec<SigningTxData>, args::Tx), TaskError> {
        let wallet = sdk.namada.wallet.read().await;
        let target_address = wallet
            .find_address(&self.target.name)
            .ok_or_else(|| TaskError::Wallet(format!("No target address: {}", self.target.name)))?;
        let fee_payer = wallet
            .find_public_key(&self.settings.gas_payer.name)
            .map_err(|e| TaskError::Wallet(e.to_string()))?;

        let mut deactivate_validator_builder_tx = sdk
            .namada
            .new_deactivate_validator(target_address.into_owned());

        deactivate_validator_builder_tx =
            deactivate_validator_builder_tx.gas_limit(GasLimit::from(self.settings.gas_limit));
        deactivate_validator_builder_tx =
            deactivate_validator_builder_tx.wrapper_fee_payer(fee_payer);

        let mut signing_keys = vec![];
        for signer in &self.settings.signers {
            let public_key = wallet
                .find_public_key(&signer.name)
                .map_err(|e| TaskError::Wallet(e.to_string()))?;
            signing_keys.push(public_key)
        }
        deactivate_validator_builder_tx =
            deactivate_validator_builder_tx.signing_keys(signing_keys);

        let (deactivate_validator, signing_data) = deactivate_validator_builder_tx
            .build(&sdk.namada)
            .await
            .map_err(|e| TaskError::BuildTx(e.to_string()))?;

        Ok((
            deactivate_validator,
            vec![signing_data],
            deactivate_validator_builder_tx.tx,
        ))
    }

    async fn build_checks(
        &self,
        _sdk: &Sdk,
        _retry_config: RetryConfig,
    ) -> Result<Vec<Check>, TaskError> {
        Ok(vec![Check::ValidatorStatus(
            check::validator_status::ValidatorStatus::builder()
                .target(self.target.clone())
                .status(ValidatorStatus::Inactive)
                .build(),
        )])
    }

    fn update_state(&self, state: &mut State) {
        state.set_validator_as_deactivated(&self.target);
    }
}
