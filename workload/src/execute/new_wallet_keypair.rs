use namada_sdk::{key::{common, SchemeType}, masp::find_valid_diversifier, masp_primitives::zip32, PaymentAddress};
use rand::rngs::OsRng;

use crate::{entities::Alias, sdk::namada::Sdk, steps::StepError};

pub async fn execute_new_wallet_key_pair(
    sdk: &Sdk,
    source_alias: Alias,
) -> Result<common::PublicKey, StepError> {
    let mut wallet = sdk.namada.wallet.write().await;

    let keypair = wallet.gen_store_secret_key(
        SchemeType::Ed25519,
        Some(source_alias.name.clone()),
        true,
        None,
        &mut OsRng,
    );

    let (_alias, sk) = if let Some((alias, sk)) = keypair {
        wallet.save().expect("unable to save wallet");
        (alias, sk)
    } else {
        return Err(StepError::Wallet("Failed to save keypair".to_string()));
    };

    let spending_key_alias = format!("{}-spending-key", source_alias.name.clone());
    let spending_key =
        wallet.gen_store_spending_key(spending_key_alias.clone(), None, None, true, &mut OsRng);

    let (_, spending_key) = if let Some((alias, sk)) = spending_key {
        wallet.save().expect("unable to save wallet");
        (alias, sk)
    } else {
        return Err(StepError::Build("Can't save spending key".to_string()))
    };

    let viewing_key = zip32::ExtendedFullViewingKey::from(&spending_key.into())
        .fvk
        .vk;
    let (div, _g_d) = find_valid_diversifier(&mut OsRng);
    let masp_payment_addr: namada_sdk::masp_primitives::sapling::PaymentAddress = viewing_key
        .to_payment_address(div)
        .expect("a PaymentAddress");
    let payment_addr = PaymentAddress::from(masp_payment_addr);

    let payment_address_alias = format!("{}-payment-address", source_alias.name);
    wallet.insert_payment_addr(payment_address_alias, payment_addr, true);
    wallet.save().expect("unable to save wallet");

    drop(wallet);

    Ok(sk.to_public())
}
