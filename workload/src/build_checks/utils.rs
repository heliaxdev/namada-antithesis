use std::str::FromStr;

use namada_sdk::{address::Address, rpc, token};
use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};

use crate::{entities::Alias, sdk::namada::Sdk};

pub async fn get_balance(
    sdk: &Sdk,
    source: Alias,
    retry_config: RetryFutureConfig<ExponentialBackoff, NoOnRetry>,
) -> Option<token::Amount> {
    let client = sdk.namada.clone_client();
    let wallet = sdk.namada.wallet.read().await;
    let native_token_address = wallet.find_address("nam").unwrap().into_owned();
    let target_address = wallet.find_address(&source.name).unwrap().into_owned();
    drop(wallet);

    tryhard::retry_fn(|| {
        rpc::get_token_balance(&client, &native_token_address, &target_address, None)
    })
    .with_config(retry_config)
    .on_retry(|attempt, _, error| {
        let error = error.to_string();
        async move {
            tracing::info!("Retry {} due to {}...", attempt, error);
        }
    })
    .await
    .ok()
}

pub async fn get_bond(
    sdk: &Sdk,
    source: Alias,
    validator: String,
    epoch: u64,
    retry_config: RetryFutureConfig<ExponentialBackoff, NoOnRetry>,
) -> Option<token::Amount> {
    let client = sdk.namada.clone_client();
    let wallet = sdk.namada.wallet.read().await;
    let source_address = wallet.find_address(&source.name).unwrap().into_owned();

    let validator_address = Address::from_str(&validator).unwrap();
    let epoch = namada_sdk::state::Epoch::from(epoch);
    drop(wallet);

    tryhard::retry_fn(|| {
        rpc::get_bond_amount_at(
            &client,
            &source_address,
            &validator_address,
            epoch.next().next(),
        )
    })
    .with_config(retry_config)
    .on_retry(|attempt, _, error| {
        let error = error.to_string();
        async move {
            tracing::info!("Retry {} due to {}...", attempt, error);
        }
    })
    .await
    .ok()
}
