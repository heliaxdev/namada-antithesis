use namada_sdk::args::{self, TxBuilder};
use namada_sdk::signing::SigningTxData;
use namada_sdk::tx::data::GasLimit;
use namada_sdk::tx::Tx;
use namada_sdk::Namada;
use typed_builder::TypedBuilder;

use crate::check::Check;
use crate::context::Ctx;
use crate::error::TaskError;
use crate::state::State;
use crate::task::{TaskContext, TaskSettings};
use crate::types::Alias;
use crate::utils::RetryConfig;

#[derive(Clone, Debug, TypedBuilder)]
pub struct ChangeMetadata {
    source: Alias,
    website: String,
    email: String,
    discord: String,
    description: String,
    avatar: String,
    settings: TaskSettings,
}

impl TaskContext for ChangeMetadata {
    fn name(&self) -> String {
        "change-metadata".to_string()
    }

    fn summary(&self) -> String {
        format!("change-metadata/{}", self.source.name)
    }

    fn task_settings(&self) -> Option<&TaskSettings> {
        Some(&self.settings)
    }

    async fn build_tx(&self, ctx: &Ctx) -> Result<(Tx, Vec<SigningTxData>, args::Tx), TaskError> {
        let wallet = ctx.namada.wallet.read().await;
        let source_address = wallet
            .find_address(&self.source.name)
            .ok_or_else(|| TaskError::Wallet(format!("No source address: {}", self.source.name)))?;
        let fee_payer = wallet
            .find_public_key(&self.settings.gas_payer.name)
            .map_err(|e| TaskError::Wallet(e.to_string()))?;

        let mut change_metadata_tx_builder = ctx
            .namada
            .new_change_metadata(source_address.into_owned())
            .avatar(self.avatar.clone())
            .description(self.description.clone())
            .discord_handle(self.discord.clone())
            .email(self.email.clone())
            .website(self.website.clone());

        change_metadata_tx_builder =
            change_metadata_tx_builder.gas_limit(GasLimit::from(self.settings.gas_limit));
        change_metadata_tx_builder = change_metadata_tx_builder.wrapper_fee_payer(fee_payer);

        let mut signing_keys = vec![];
        for signer in &self.settings.signers {
            let public_key = wallet
                .find_public_key(&signer.name)
                .map_err(|e| TaskError::Wallet(e.to_string()))?;
            signing_keys.push(public_key)
        }
        change_metadata_tx_builder = change_metadata_tx_builder.signing_keys(signing_keys);
        drop(wallet);

        let (change_metadata, signing_data) = change_metadata_tx_builder
            .build(&ctx.namada)
            .await
            .map_err(|e| TaskError::BuildTx(e.to_string()))?;

        Ok((
            change_metadata,
            vec![signing_data],
            change_metadata_tx_builder.tx,
        ))
    }

    async fn build_checks(
        &self,
        _ctx: &Ctx,
        _retry_config: RetryConfig,
    ) -> Result<Vec<Check>, TaskError> {
        Ok(vec![])
    }

    fn update_state(&self, _state: &mut State) {}
}
