use namada_sdk::{
    args::{self, TxBuilder},
    signing::SigningTxData,
    tx::{data::GasLimit, Tx},
    Namada,
};

use crate::{entities::Alias, sdk::namada::Sdk, steps::StepError, task::TaskSettings};

use super::utils;

#[allow(clippy::too_many_arguments)]
pub async fn build_tx_change_metadata(
    sdk: &Sdk,
    source: Alias,
    website: String,
    email: String,
    discord: String,
    description: String,
    avatar: String,
    settings: TaskSettings,
) -> Result<(Tx, SigningTxData, args::Tx), StepError> {
    let wallet = sdk.namada.wallet.read().await;
    let source_address = wallet.find_address(source.name).unwrap().into_owned();
    let fee_payer = wallet.find_public_key(&settings.gas_payer.name).unwrap();

    let mut change_metadata_tx_builder = sdk
        .namada
        .new_change_metadata(source_address.clone())
        .avatar(avatar)
        .description(description)
        .discord_handle(discord)
        .email(email)
        .website(website);

    change_metadata_tx_builder =
        change_metadata_tx_builder.gas_limit(GasLimit::from(settings.gas_limit));
    change_metadata_tx_builder = change_metadata_tx_builder.wrapper_fee_payer(fee_payer);

    let mut signing_keys = vec![];
    for signer in settings.signers {
        let public_key = wallet.find_public_key(&signer.name).unwrap();
        signing_keys.push(public_key)
    }
    change_metadata_tx_builder = change_metadata_tx_builder.signing_keys(signing_keys.clone());

    let (change_metadata, signing_data) = change_metadata_tx_builder
        .build(&sdk.namada)
        .await
        .map_err(|e| StepError::Build(e.to_string()))?;

    Ok((change_metadata, signing_data, change_metadata_tx_builder.tx))
}

pub async fn execute_tx_become_validator(
    sdk: &Sdk,
    tx: &mut Tx,
    signing_data: SigningTxData,
    tx_args: &args::Tx,
) -> Result<Option<u64>, StepError> {
    utils::execute_tx(sdk, tx, vec![signing_data], tx_args).await
}
