use namada_sdk::{
    args::{self, InputAmount, TxExpiration, TxUnshieldingTransferData},
    signing::SigningTxData,
    token::{self, DenominatedAmount},
    tx::Tx,
    Namada,
};

use crate::{entities::Alias, sdk::namada::Sdk, steps::StepError, task::TaskSettings};

use super::utils;

pub async fn build_tx_unshielding(
    sdk: &Sdk,
    source: Alias,
    target: Alias,
    amount: u64,
    settings: TaskSettings,
) -> Result<(Tx, SigningTxData, args::Tx), StepError> {
    let mut wallet = sdk.namada.wallet.write().await;

    let native_token_alias = Alias::nam();
    let token_address = wallet
        .find_address(native_token_alias.name)
        .unwrap()
        .as_ref()
        .clone();
    let token_amount = token::Amount::from_u64(amount);


    let spending_key_alias = Alias { name: format!("{}-spending-key", source.name.strip_suffix("-payment-address").unwrap()) };
    let source_spending_key = wallet
        .find_spending_key(spending_key_alias.name, None)
        .unwrap()
        .key;
    let target_payment_address = wallet.find_address(target.name).unwrap().clone();

    let tx_transfer_data = TxUnshieldingTransferData {
        target: target_payment_address.into_owned(),
        token: token_address.clone(),
        amount: InputAmount::Validated(DenominatedAmount::native(token_amount)),
    };

    let mut transfer_tx_builder = sdk.namada.new_unshielding_transfer(
        source_spending_key,
        vec![tx_transfer_data],
        None,
        true,
    );
    
    // // //let gas_payer = wallet.find_public_key(&settings.gas_payer.name).unwrap();
    // // transfer_tx_builder.tx.signing_keys = signing_keys; //vec![gas_payer.clone()];
    // // transfer_tx_builder.tx.expiration = TxExpiration::NoExpiration;

    let (transfer_tx, signing_data) = transfer_tx_builder.build(&sdk.namada)
        .await
        .map_err(|e| StepError::Build(e.to_string()))?;

    // transfer_tx_builder.tx.signing_keys = signing_keys; //vec![gas_payer.clone()];
    // transfer_tx_builder.tx.expiration = TxExpiration::NoExpiration;
    

    Ok((transfer_tx, signing_data, transfer_tx_builder.tx))
}

pub async fn execute_tx_unshielding(
    sdk: &Sdk,
    tx: &mut Tx,
    signing_data: SigningTxData,
    tx_args: &args::Tx,
) -> Result<Option<u64>, StepError> {
    utils::execute_tx(sdk, tx, vec![signing_data], tx_args).await
}
