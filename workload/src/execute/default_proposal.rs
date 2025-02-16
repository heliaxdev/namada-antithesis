use std::collections::BTreeMap;

use namada_sdk::governance::cli::onchain::{DefaultProposal, OnChainProposal};
use namada_sdk::{
    args::{self, TxBuilder},
    signing::SigningTxData,
    tx::{data::GasLimit, Tx},
    Namada,
};

use crate::{entities::Alias, sdk::namada::Sdk, steps::StepError, task::TaskSettings};

use super::utils::execute_tx;

#[allow(clippy::too_many_arguments)]
pub async fn build_tx_default_proposal(
    sdk: &Sdk,
    source: &Alias,
    start_epoch: u64,
    end_epoch: u64,
    grace_epoch: u64,
    settings: &TaskSettings,
) -> Result<(Tx, SigningTxData, args::Tx), StepError> {
    let wallet = sdk.namada.wallet.read().await;
    let source_address = wallet
        .find_address(&source.name)
        .ok_or_else(|| StepError::Wallet(format!("No source address: {}", source.name)))?;
    let fee_payer = wallet
        .find_public_key(&settings.gas_payer.name)
        .map_err(|e| StepError::Wallet(e.to_string()))?;

    let default_proposal = DefaultProposal {
        proposal: OnChainProposal {
            content: BTreeMap::from_iter([("workload".to_string(), "tester".to_string())]),
            author: source_address.into_owned(),
            voting_start_epoch: start_epoch.into(),
            voting_end_epoch: end_epoch.into(),
            activation_epoch: grace_epoch.into(),
        },
        data: if start_epoch % 2 == 0 {
            Some(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10])
        } else {
            None
        },
    };
    let proposal_json =
        serde_json::to_string(&default_proposal).expect("Encoding proposal shouldn't fail");

    let mut default_proposal_tx_builder = sdk.namada.new_init_proposal(proposal_json.into_bytes());

    default_proposal_tx_builder =
        default_proposal_tx_builder.gas_limit(GasLimit::from(settings.gas_limit));
    default_proposal_tx_builder = default_proposal_tx_builder.wrapper_fee_payer(fee_payer);

    let mut signing_keys = vec![];
    for signer in &settings.signers {
        let public_key = wallet
            .find_public_key(&signer.name)
            .map_err(|e| StepError::Wallet(e.to_string()))?;
        signing_keys.push(public_key)
    }
    default_proposal_tx_builder = default_proposal_tx_builder.signing_keys(signing_keys);
    drop(wallet);

    let (default_proposal, signing_data) = default_proposal_tx_builder
        .build(&sdk.namada)
        .await
        .map_err(|e| StepError::Build(e.to_string()))?;

    Ok((
        default_proposal,
        signing_data,
        default_proposal_tx_builder.tx,
    ))
}

pub async fn execute_tx_default_proposal(
    sdk: &Sdk,
    source: &Alias,
    start_epoch: u64,
    end_epoch: u64,
    grace_epoch: u64,
    settings: &TaskSettings,
) -> Result<Option<u64>, StepError> {
    let (default_proposal_tx, signing_data, tx_args) =
        build_tx_default_proposal(sdk, source, start_epoch, end_epoch, grace_epoch, settings)
            .await?;

    execute_tx(sdk, default_proposal_tx, vec![signing_data], &tx_args).await
}
