use namada_sdk::key::{common, SchemeType};
use rand::rngs::OsRng;

use crate::{entities::Alias, sdk::namada::Sdk, steps::StepError};

pub async fn execute_new_wallet_key_pair(
    sdk: &Sdk,
    source_alias: Alias,
) -> Result<common::PublicKey, StepError> {
    let mut wallet = sdk.namada.wallet.write().await;

    let keypair = wallet.gen_store_secret_key(
        SchemeType::Ed25519,
        Some(source_alias.name),
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
    drop(wallet);

    Ok(sk.to_public())
}
