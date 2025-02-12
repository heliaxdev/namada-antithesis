use namada_sdk::{
    args::{self, TxBuilder},
    signing::SigningTxData,
    tx::{data::GasLimit, Tx},
    Namada,
};

use crate::{entities::Alias, sdk::namada::Sdk, steps::StepError, task::TaskSettings};

use super::utils;

pub async fn build_tx_reactivate_validator(
    sdk: &Sdk,
    source: Alias,
    settings: TaskSettings,
) -> Result<(Tx, SigningTxData, args::Tx), StepError> {
    let wallet = sdk.namada.wallet.read().await;
    let source_address = wallet.find_address(source.name).unwrap().into_owned();
    let fee_payer = wallet.find_public_key(&settings.gas_payer.name).unwrap();

    let mut reactivate_validator_builder_tx =
        sdk.namada.new_reactivate_validator(source_address.clone());

    reactivate_validator_builder_tx =
        reactivate_validator_builder_tx.gas_limit(GasLimit::from(settings.gas_limit));
    reactivate_validator_builder_tx = reactivate_validator_builder_tx.wrapper_fee_payer(fee_payer);

    let mut signing_keys = vec![];
    for signer in settings.signers {
        let public_key = wallet.find_public_key(&signer.name).unwrap();
        signing_keys.push(public_key)
    }
    reactivate_validator_builder_tx =
        reactivate_validator_builder_tx.signing_keys(signing_keys.clone());

    let (reactivate_validator, signing_data) = reactivate_validator_builder_tx
        .build(&sdk.namada)
        .await
        .map_err(|e| StepError::Build(e.to_string()))?;

    Ok((
        reactivate_validator,
        signing_data,
        reactivate_validator_builder_tx.tx,
    ))
}

pub async fn execute_tx_reactivate_validator(
    sdk: &Sdk,
    tx: &mut Tx,
    signing_data: SigningTxData,
    tx_args: &args::Tx,
) -> Result<Option<u64>, StepError> {
    utils::execute_tx(sdk, tx, vec![signing_data], tx_args).await
}
