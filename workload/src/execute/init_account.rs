use std::collections::BTreeSet;

use namada_sdk::{
    args::{self, TxBuilder},
    signing::SigningTxData,
    tx::{data::GasLimit, Tx},
    Namada,
};

use crate::{entities::Alias, sdk::namada::Sdk, steps::StepError, task::TaskSettings};

pub async fn build_tx_init_account(
    sdk: &Sdk,
    target: Alias,
    sources: BTreeSet<Alias>,
    threshold: u64,
    settings: TaskSettings,
) -> Result<(Tx, SigningTxData, args::Tx), StepError> {
    let wallet = sdk.namada.wallet.write().await;

    let mut public_keys = vec![];
    for source in sources {
        let source_pk = wallet.find_public_key(source.name).unwrap();
        public_keys.push(source_pk);
    }

    let fee_payer = wallet.find_public_key(&settings.gas_payer.name).unwrap();

    let mut init_account_builder = sdk
        .namada
        .new_init_account(public_keys, Some(threshold as u8))
        .initialized_account_alias(target.name);

    init_account_builder = init_account_builder.gas_limit(GasLimit::from(settings.gas_limit));
    init_account_builder = init_account_builder.wrapper_fee_payer(fee_payer);

    let mut signing_keys = vec![];
    for signer in settings.signers {
        let public_key = wallet.find_public_key(&signer.name).unwrap();
        signing_keys.push(public_key)
    }
    init_account_builder = init_account_builder.signing_keys(signing_keys.clone());
    drop(wallet);

    let (init_account, signing_data) = init_account_builder
        .build(&sdk.namada)
        .await
        .map_err(|e| StepError::Build(e.to_string()))?;

    Ok((init_account, signing_data, init_account_builder.tx))
}
