use std::str::FromStr;

use namada_sdk::{
    address::Address,
    args::{self, TxBuilder},
    signing::SigningTxData,
    tx::{data::GasLimit, Tx},
    Namada,
};

use crate::{entities::Alias, sdk::namada::Sdk, steps::StepError, task::TaskSettings};

use super::utils;

pub async fn build_tx_claim_rewards(
    sdk: &Sdk,
    source: Alias,
    from_validator: String,
    settings: TaskSettings,
) -> Result<(Tx, SigningTxData, args::Tx), StepError> {
    let wallet = sdk.namada.wallet.write().await;

    let source_address = wallet.find_address(source.name).unwrap().as_ref().clone();
    let fee_payer = wallet.find_public_key(&settings.gas_payer.name).unwrap();
    let from_validator = Address::from_str(&from_validator).unwrap(); // safe

    let mut claim_rewards_tx_builder = sdk.namada.new_claim_rewards(from_validator);
    claim_rewards_tx_builder.source = Some(source_address);
    claim_rewards_tx_builder =
        claim_rewards_tx_builder.gas_limit(GasLimit::from(settings.gas_limit));
    claim_rewards_tx_builder = claim_rewards_tx_builder.wrapper_fee_payer(fee_payer);
    let mut signing_keys = vec![];
    for signer in settings.signers {
        let public_key = wallet.find_public_key(&signer.name).unwrap();
        signing_keys.push(public_key)
    }
    claim_rewards_tx_builder = claim_rewards_tx_builder.signing_keys(signing_keys.clone());
    drop(wallet);

    let (claim_tx, signing_data) = claim_rewards_tx_builder
        .build(&sdk.namada)
        .await
        .map_err(|e| StepError::Build(e.to_string()))?;

    Ok((claim_tx, signing_data, claim_rewards_tx_builder.tx))
}

pub async fn execute_tx_claim_rewards(
    sdk: &Sdk,
    tx: &mut Tx,
    signing_data: SigningTxData,
    tx_args: &args::Tx,
) -> Result<Option<u64>, StepError> {
    utils::execute_tx(sdk, tx, vec![signing_data], tx_args).await
}
