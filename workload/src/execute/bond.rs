use std::str::FromStr;

use namada_sdk::{address::Address, args::TxBuilder, rpc::TxResponse, signing::default_sign, token, tx::{data::GasLimit, ProcessTxResponse}, Namada};

use crate::{entities::Alias, sdk::namada::Sdk, steps::StepError, task::TaskSettings};

use super::utils;

pub async fn execute_bond(
    sdk: &Sdk,
    source: Alias,
    validator: String,
    amount: u64,
    settings: TaskSettings,
) -> Result<Option<u64>, StepError> {
    let wallet = sdk.namada.wallet.write().await;

    let source_address = wallet.find_address(source.name).unwrap().as_ref().clone();
    let token_amount = token::Amount::from_u64(amount);
    let fee_payer = wallet.find_public_key(&settings.gas_payer.name).unwrap();
    let validator = Address::from_str(&validator).unwrap(); // safe

    let mut bond_tx_builder = sdk
        .namada
        .new_bond(validator, token_amount)
        .source(source_address);
    bond_tx_builder = bond_tx_builder.gas_limit(GasLimit::from(settings.gas_limit));
    bond_tx_builder = bond_tx_builder.wrapper_fee_payer(fee_payer);
    let mut signing_keys = vec![];
    for signer in settings.signers {
        let public_key = wallet.find_public_key(&signer.name).unwrap();
        signing_keys.push(public_key)
    }
    bond_tx_builder = bond_tx_builder.signing_keys(signing_keys.clone());
    drop(wallet);

    let (mut bond_tx, signing_data) = bond_tx_builder
        .build(&sdk.namada)
        .await
        .map_err(|e| StepError::Build(e.to_string()))?;

    sdk.namada
        .sign(
            &mut bond_tx,
            &bond_tx_builder.tx,
            signing_data,
            default_sign,
            (),
        )
        .await
        .expect("unable to sign tx");

    let tx = sdk
        .namada
        .submit(bond_tx.clone(), &bond_tx_builder.tx)
        .await;

    let execution_height = if let Ok(ProcessTxResponse::Applied(TxResponse { height, .. })) = &tx {
        Some(height.0)
    } else {
        None
    };

    if utils::is_tx_rejected(&bond_tx, &tx) {
        match tx {
            Ok(tx) => {
                let errors = utils::get_tx_errors(&bond_tx, &tx).unwrap_or_default();
                return Err(StepError::Execution(errors));
            }
            Err(e) => return Err(StepError::Broadcast(e.to_string())),
        }
    }
    
    Ok(execution_height)
}
