use namada_sdk::{
    args::{self, TxBuilder},
    signing::SigningTxData,
    tx::{data::GasLimit, Tx},
    Namada,
};

use crate::{entities::Alias, sdk::namada::Sdk, steps::StepError, task::TaskSettings};

use super::utils;

pub async fn build_tx_vote(
    sdk: &Sdk,
    source: Alias,
    proposal_id: u64,
    vote: String,
    settings: TaskSettings,
) -> Result<(Tx, SigningTxData, args::Tx), StepError> {
    let wallet = sdk.namada.wallet.write().await;
    let source_address = wallet.find_address(source.name).unwrap().as_ref().clone();
    let fee_payer = wallet.find_public_key(&settings.gas_payer.name).unwrap();

    let mut vote_tx_builder = sdk
        .namada
        .new_proposal_vote(proposal_id, vote, source_address);
    vote_tx_builder = vote_tx_builder.gas_limit(GasLimit::from(settings.gas_limit));
    vote_tx_builder = vote_tx_builder.wrapper_fee_payer(fee_payer);
    let mut signing_keys = vec![];
    for signer in settings.signers {
        let public_key = wallet.find_public_key(&signer.name).unwrap();
        signing_keys.push(public_key)
    }
    vote_tx_builder = vote_tx_builder.signing_keys(signing_keys.clone());
    drop(wallet);

    let (vote_tx, signing_data) = vote_tx_builder
        .build(&sdk.namada)
        .await
        .map_err(|e| StepError::Build(e.to_string()))?;

    Ok((vote_tx, signing_data, vote_tx_builder.tx))
}

pub async fn execute_tx_vote(
    sdk: &Sdk,
    tx: &mut Tx,
    signing_data: SigningTxData,
    tx_args: &args::Tx,
) -> Result<Option<u64>, StepError> {
    utils::execute_tx(sdk, tx, vec![signing_data], tx_args).await
}
