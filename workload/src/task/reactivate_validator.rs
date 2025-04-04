use namada_sdk::args::{self, TxBuilder};
use namada_sdk::signing::SigningTxData;
use namada_sdk::tx::data::GasLimit;
use namada_sdk::tx::Tx;
use namada_sdk::Namada;
use typed_builder::TypedBuilder;

use crate::check::{self, Check};
use crate::context::Ctx;
use crate::error::TaskError;
use crate::state::State;
use crate::task::{TaskContext, TaskSettings};
use crate::types::{Alias, ValidatorStatus};
use crate::utils::RetryConfig;

#[derive(Clone, Debug, TypedBuilder)]
pub struct ReactivateValidator {
    target: Alias,
    settings: TaskSettings,
}

impl TaskContext for ReactivateValidator {
    fn name(&self) -> String {
        "reactivate-validator".to_string()
    }

    fn summary(&self) -> String {
        format!("reactivate-validator/{}", self.target.name)
    }

    fn task_settings(&self) -> Option<&TaskSettings> {
        Some(&self.settings)
    }

    async fn build_tx(&self, ctx: &Ctx) -> Result<(Tx, Vec<SigningTxData>, args::Tx), TaskError> {
        let wallet = ctx.namada.wallet.read().await;
        let target_address = wallet
            .find_address(&self.target.name)
            .ok_or_else(|| TaskError::Wallet(format!("No target address: {}", self.target.name)))?;
        let fee_payer = wallet
            .find_public_key(&self.settings.gas_payer.name)
            .map_err(|e| TaskError::Wallet(e.to_string()))?;

        let mut reactivate_validator_builder_tx = ctx
            .namada
            .new_reactivate_validator(target_address.into_owned());

        reactivate_validator_builder_tx =
            reactivate_validator_builder_tx.gas_limit(GasLimit::from(self.settings.gas_limit));
        reactivate_validator_builder_tx =
            reactivate_validator_builder_tx.wrapper_fee_payer(fee_payer);

        let mut signing_keys = vec![];
        for signer in &self.settings.signers {
            let public_key = wallet
                .find_public_key(&signer.name)
                .map_err(|e| TaskError::Wallet(e.to_string()))?;
            signing_keys.push(public_key)
        }
        reactivate_validator_builder_tx =
            reactivate_validator_builder_tx.signing_keys(signing_keys);

        let (reactivate_validator, signing_data) = reactivate_validator_builder_tx
            .build(&ctx.namada)
            .await
            .map_err(|e| TaskError::BuildTx(e.to_string()))?;

        Ok((
            reactivate_validator,
            vec![signing_data],
            reactivate_validator_builder_tx.tx,
        ))
    }

    async fn build_checks(
        &self,
        _ctx: &Ctx,
        _retry_config: RetryConfig,
    ) -> Result<Vec<Check>, TaskError> {
        Ok(vec![Check::ValidatorStatus(
            check::validator_status::ValidatorStatus::builder()
                .target(self.target.clone())
                .status(ValidatorStatus::Reactivating)
                .build(),
        )])
    }

    fn update_state(&self, state: &mut State) {
        state.reactivate_validator(&self.target);
    }
}
