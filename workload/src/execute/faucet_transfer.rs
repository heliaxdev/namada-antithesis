use namada_sdk::{
    args::{InputAmount, TxBuilder, TxTransparentTransferData},
    rpc::TxResponse,
    signing::default_sign,
    token::{self, DenominatedAmount},
    tx::{data::GasLimit, ProcessTxResponse},
    Namada,
};

use crate::{entities::Alias, sdk::namada::Sdk, steps::StepError, task::TaskSettings};

use super::utils;

pub async fn execute_faucet_transfer(
    sdk: &Sdk,
    target: Alias,
    amount: u64,
    settings: TaskSettings,
) -> Result<Option<u64>, StepError> {
    let wallet = sdk.namada.wallet.write().await;

    let faucet_alias = Alias::faucet();
    let native_token_alias = Alias::nam();

    let source_address = wallet
        .find_address(faucet_alias.name)
        .unwrap()
        .as_ref()
        .clone();
    let target_address = wallet.find_address(target.name).unwrap().as_ref().clone();
    let token_address = wallet
        .find_address(native_token_alias.name)
        .unwrap()
        .as_ref()
        .clone();
    let fee_payer = wallet.find_public_key(&settings.gas_payer.name).unwrap();
    let token_amount = token::Amount::from_u64(amount);

    let tx_transfer_data = TxTransparentTransferData {
        source: source_address.clone(),
        target: target_address.clone(),
        token: token_address,
        amount: InputAmount::Unvalidated(DenominatedAmount::native(token_amount)),
    };

    let mut transfer_tx_builder = sdk.namada.new_transparent_transfer(vec![tx_transfer_data]);

    transfer_tx_builder = transfer_tx_builder.gas_limit(GasLimit::from(settings.gas_limit));
    transfer_tx_builder = transfer_tx_builder.wrapper_fee_payer(fee_payer);

    let mut signing_keys = vec![];
    for signer in settings.signers {
        let public_key = wallet.find_public_key(&signer.name).unwrap();
        signing_keys.push(public_key)
    }
    transfer_tx_builder = transfer_tx_builder.signing_keys(signing_keys.clone());
    drop(wallet);

    let (mut transfer_tx, signing_data) = transfer_tx_builder
        .build(&sdk.namada)
        .await
        .map_err(|e| StepError::Build(e.to_string()))?;

    sdk.namada
        .sign(
            &mut transfer_tx,
            &transfer_tx_builder.tx,
            signing_data,
            default_sign,
            (),
        )
        .await
        .expect("unable to sign tx");

    let tx = sdk
        .namada
        .submit(transfer_tx.clone(), &transfer_tx_builder.tx)
        .await;

    let execution_height = if let Ok(ProcessTxResponse::Applied(TxResponse { height, .. })) = &tx {
        Some(height.0)
    } else {
        None
    };

    if utils::is_tx_rejected(&transfer_tx, &tx) {
        match tx {
            Ok(tx) => {
                let errors = utils::get_tx_errors(&transfer_tx, &tx).unwrap_or_default();
                return Err(StepError::Execution(errors));
            }
            Err(e) => return Err(StepError::Broadcast(e.to_string())),
        }
    }

    Ok(execution_height)
}
