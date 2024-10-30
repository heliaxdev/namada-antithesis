use namada_sdk::{
    args::{self, InputAmount, TxBuilder, TxTransparentTransferData},
    rpc::TxResponse,
    signing::{default_sign, SigningTxData},
    token::{self, DenominatedAmount},
    tx::{data::GasLimit, ProcessTxResponse, Tx},
    Namada,
};

use crate::{entities::Alias, sdk::namada::Sdk, steps::StepError, task::TaskSettings};

use super::utils;

pub async fn build_tx_transparent_transfer(
    sdk: &Sdk,
    source: Alias,
    target: Alias,
    amount: u64,
    settings: TaskSettings,
) -> Result<(Tx, SigningTxData, args::Tx), StepError> {
    let wallet = sdk.namada.wallet.write().await;

    let native_token_alias = Alias::nam();

    let source_address = wallet.find_address(source.name).unwrap().as_ref().clone();
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

    let (transfer_tx, signing_data) = transfer_tx_builder
        .build(&sdk.namada)
        .await
        .map_err(|e| StepError::Build(e.to_string()))?;

    Ok((transfer_tx, signing_data, transfer_tx_builder.tx))
}

pub async fn execute_tx_transparent_transfer(
    sdk: &Sdk,
    tx: &mut Tx,
    signing_data: SigningTxData,
    tx_args: &args::Tx
) -> Result<Option<u64>, StepError> {
    utils::execute_tx(sdk, tx, vec![signing_data], tx_args).await
}