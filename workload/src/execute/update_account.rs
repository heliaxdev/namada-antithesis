use std::collections::BTreeSet;

use namada_sdk::{
    args::{self, TxBuilder},
    signing::SigningTxData,
    tx::{data::GasLimit, Tx},
    Namada,
};

use crate::{entities::Alias, sdk::namada::Sdk, steps::StepError, task::TaskSettings};

use super::utils;

pub async fn build_tx_update_account(
    sdk: &Sdk,
    target: Alias,
    sources: BTreeSet<Alias>,
    threshold: u64,
    settings: TaskSettings,
) -> Result<(Tx, SigningTxData, args::Tx), StepError> {
    let wallet = sdk.namada.wallet.write().await;
    let target = wallet.find_address(target.name).unwrap().into_owned();

    let mut public_keys = vec![];
    for source in sources {
        let source_pk = wallet.find_public_key(source.name).unwrap();
        public_keys.push(source_pk);
    }

    let fee_payer = wallet.find_public_key(&settings.gas_payer.name).unwrap();

    let mut update_account_builder =
        sdk.namada
            .new_update_account(target, public_keys, threshold as u8);

    update_account_builder = update_account_builder.gas_limit(GasLimit::from(settings.gas_limit));
    update_account_builder = update_account_builder.wrapper_fee_payer(fee_payer);

    let mut signing_keys = vec![];
    for signer in settings.signers {
        let public_key = wallet.find_public_key(&signer.name).unwrap();
        signing_keys.push(public_key)
    }
    update_account_builder = update_account_builder.signing_keys(signing_keys.clone());
    drop(wallet);

    let (update_account, signing_data) = update_account_builder
        .build(&sdk.namada)
        .await
        .map_err(|e| StepError::Build(e.to_string()))?;

    Ok((update_account, signing_data, update_account_builder.tx))
}

pub async fn execute_tx_update_account(
    sdk: &Sdk,
    tx: &mut Tx,
    signing_data: SigningTxData,
    tx_args: &args::Tx,
) -> Result<Option<u64>, StepError> {
    utils::execute_tx(sdk, tx, vec![signing_data], tx_args).await
}
