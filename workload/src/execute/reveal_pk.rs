use namada_sdk::{
    args::TxBuilder, key::common, rpc::TxResponse, signing::default_sign, tx::ProcessTxResponse,
    Namada,
};

use crate::{sdk::namada::Sdk, steps::StepError};

use super::utils;

pub async fn execute_reveal_pk(
    sdk: &Sdk,
    public_key: common::PublicKey,
) -> Result<Option<u64>, StepError> {
    let wallet = sdk.namada.wallet.write().await;
    let fee_payer = wallet.find_public_key("faucet").unwrap();
    drop(wallet);

    let reveal_pk_tx_builder = sdk
        .namada
        .new_reveal_pk(public_key.clone())
        .signing_keys(vec![public_key.clone()])
        .wrapper_fee_payer(fee_payer);

    let (mut reveal_tx, signing_data) = reveal_pk_tx_builder
        .build(&sdk.namada)
        .await
        .map_err(|e| StepError::Build(e.to_string()))?;

    sdk.namada
        .sign(
            &mut reveal_tx,
            &reveal_pk_tx_builder.tx,
            signing_data,
            default_sign,
            (),
        )
        .await
        .expect("unable to sign tx");

    let tx = sdk
        .namada
        .submit(reveal_tx.clone(), &reveal_pk_tx_builder.tx)
        .await;

    if utils::is_tx_rejected(&reveal_tx, &tx) {
        match tx {
            Ok(tx) => {
                let errors = utils::get_tx_errors(&reveal_tx, &tx).unwrap_or_default();
                return Err(StepError::Execution(errors));
            }
            Err(e) => return Err(StepError::Broadcast(e.to_string())),
        }
    }

    if let Ok(ProcessTxResponse::Applied(TxResponse { height, .. })) = &tx {
        Ok(Some(height.0))
    } else {
        Ok(None)
    }
}
