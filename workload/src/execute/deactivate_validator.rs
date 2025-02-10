use namada_sdk::{
    args::{self, TxBuilder},
    signing::SigningTxData,
    tx::{data::GasLimit, Tx},
    Namada,
};

use crate::{entities::Alias, sdk::namada::Sdk, steps::StepError, task::TaskSettings};

use super::utils;

pub async fn build_tx_deactivate_validator(
    sdk: &Sdk,
    source: Alias,
    settings: TaskSettings,
) -> Result<(Tx, SigningTxData, args::Tx), StepError> {
    let wallet = sdk.namada.wallet.read().await;
    let source_address = wallet.find_address(source.name).unwrap().into_owned();
    let fee_payer = wallet.find_public_key(&settings.gas_payer.name).unwrap();

    let mut deactivate_validator_builder_tx = sdk
        .namada
        .new_deactivate_validator(source_address.clone());

    deactivate_validator_builder_tx =
        deactivate_validator_builder_tx.gas_limit(GasLimit::from(settings.gas_limit));
    deactivate_validator_builder_tx = deactivate_validator_builder_tx.wrapper_fee_payer(fee_payer);

    let mut signing_keys = vec![];
    for signer in settings.signers {
        let public_key = wallet.find_public_key(&signer.name).unwrap();
        signing_keys.push(public_key)
    }
    deactivate_validator_builder_tx = deactivate_validator_builder_tx.signing_keys(signing_keys.clone());

    let (deactivate_validator, signing_data) = deactivate_validator_builder_tx
        .build(&sdk.namada)
        .await
        .map_err(|e| StepError::Build(e.to_string()))?;

    Ok((
        deactivate_validator,
        signing_data,
        deactivate_validator_builder_tx.tx,
    ))
}

pub async fn execute_tx_deactivate_validator(
    sdk: &Sdk,
    tx: &mut Tx,
    signing_data: SigningTxData,
    tx_args: &args::Tx,
) -> Result<Option<u64>, StepError> {
    utils::execute_tx(sdk, tx, vec![signing_data], tx_args).await
}
