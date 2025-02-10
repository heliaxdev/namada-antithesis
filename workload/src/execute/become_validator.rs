use namada_sdk::key::RefTo;
use namada_sdk::{
    args::{self, TxBuilder},
    dec::Dec,
    key::SchemeType,
    signing::SigningTxData,
    tx::{data::GasLimit, Tx},
    Namada,
};
use rand::rngs::OsRng;

use crate::{entities::Alias, sdk::namada::Sdk, steps::StepError, task::TaskSettings};

use super::utils;

#[allow(clippy::too_many_arguments)]
pub async fn build_tx_become_validator(
    sdk: &Sdk,
    source: Alias,
    consensus_alias: Alias,
    eth_cold_alias: Alias,
    eth_hot_alias: Alias,
    protocol_alias: Alias,
    commission_rate: Dec,
    commission_max_change: Dec,
    settings: TaskSettings,
) -> Result<(Tx, SigningTxData, args::Tx), StepError> {
    let mut wallet = sdk.namada.wallet.write().await;

    let source_address = wallet.find_address(source.name).unwrap().into_owned();

    let consensus_pk = wallet
        .gen_store_secret_key(
            SchemeType::Ed25519,
            Some(consensus_alias.name.clone()),
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
            Some(eth_cold_alias.name.clone()),
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
            Some(eth_hot_alias.name.clone()),
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
            Some(protocol_alias.name.clone()),
            true,
            None,
            &mut OsRng,
        )
        .expect("Key generation should not fail.")
        .1
        .ref_to();

    let fee_payer = wallet.find_public_key(&settings.gas_payer.name).unwrap();
    wallet.save().expect("unable to save wallet");

    let mut become_validator_tx_builder = sdk
        .namada
        .new_become_validator(
            source_address.clone(),
            commission_rate,
            commission_max_change,
            consensus_pk,
            eth_cold_pk,
            eth_hot_pk,
            protocol_key,
            "test@test.it".to_string(),
        )
        .wallet_alias_force(true);

    become_validator_tx_builder =
        become_validator_tx_builder.gas_limit(GasLimit::from(settings.gas_limit));
    become_validator_tx_builder = become_validator_tx_builder.wrapper_fee_payer(fee_payer);

    let mut signing_keys = vec![];
    for signer in settings.signers {
        let public_key = wallet.find_public_key(&signer.name).unwrap();
        signing_keys.push(public_key)
    }
    become_validator_tx_builder = become_validator_tx_builder.signing_keys(signing_keys.clone());
    drop(wallet);

    let (become_validator, signing_data) = become_validator_tx_builder
        .build(&sdk.namada)
        .await
        .map_err(|e| StepError::Build(e.to_string()))?;

    Ok((
        become_validator,
        signing_data,
        become_validator_tx_builder.tx,
    ))
}

pub async fn execute_tx_become_validator(
    sdk: &Sdk,
    tx: &mut Tx,
    signing_data: SigningTxData,
    tx_args: &args::Tx,
) -> Result<Option<u64>, StepError> {
    utils::execute_tx(sdk, tx, vec![signing_data], tx_args).await
}
