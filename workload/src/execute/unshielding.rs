use namada_sdk::{
    args::{self, InputAmount, TxBuilder, TxUnshieldingTransferData},
    masp_primitives::{
        self, transaction::components::sapling::builder::RngBuildParams, zip32::PseudoExtendedKey,
    },
    signing::SigningTxData,
    token::{self, DenominatedAmount},
    tx::{data::GasLimit, Tx},
    Namada,
};
use rand::rngs::OsRng;

use crate::{entities::Alias, sdk::namada::Sdk, steps::StepError, task::TaskSettings};

use super::utils;

pub async fn build_tx_unshielding(
    sdk: &Sdk,
    source: Alias,
    target: Alias,
    amount: u64,
    settings: TaskSettings,
) -> Result<(Tx, SigningTxData, args::Tx), StepError> {
    let mut bparams = RngBuildParams::new(OsRng);

    let mut wallet = sdk.namada.wallet.write().await;

    let native_token_alias = Alias::nam();
    let token_address = wallet
        .find_address(native_token_alias.name)
        .unwrap()
        .as_ref()
        .clone();
    let fee_payer = wallet.find_public_key(&settings.gas_payer.name).unwrap();
    let token_amount = token::Amount::from_u64(amount);

    let spending_key_alias = Alias {
        name: format!(
            "{}-spending-key",
            source.name.strip_suffix("-payment-address").unwrap()
        ),
    };
    let source_spending_key = wallet
        .find_spending_key(spending_key_alias.name, None)
        .unwrap();

    let tmp = masp_primitives::zip32::ExtendedSpendingKey::from(source_spending_key);
    let pseudo_spending_key_from_spending_key = PseudoExtendedKey::from(tmp);
    let target_address = wallet.find_address(target.name).unwrap().clone();

    let tx_transfer_data = TxUnshieldingTransferData {
        target: target_address.into_owned(),
        token: token_address.clone(),
        amount: InputAmount::Validated(DenominatedAmount::native(token_amount)),
    };

    let mut transfer_tx_builder = sdk.namada.new_unshielding_transfer(
        pseudo_spending_key_from_spending_key,
        vec![tx_transfer_data],
        None,
        false,
    );

    transfer_tx_builder = transfer_tx_builder.gas_limit(GasLimit::from(settings.gas_limit));
    transfer_tx_builder = transfer_tx_builder.wrapper_fee_payer(fee_payer);
    let mut signing_keys = vec![];
    for signer in settings.signers {
        let public_key = wallet.find_public_key(&signer.name).unwrap();
        signing_keys.push(public_key)
    }
    transfer_tx_builder = transfer_tx_builder.signing_keys(signing_keys.clone());

    let (transfer_tx, signing_data) = transfer_tx_builder
        .build(&sdk.namada, &mut bparams)
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
