use std::str::FromStr;

use namada_sdk::{
    address::Address,
    control_flow::install_shutdown_signal,
    io::DevNullProgressBar,
    masp::{shielded_wallet::ShieldedApi, LedgerMaspClient, MaspLocalTaskEnv, ShieldedSyncConfig},
    masp_primitives::{transaction::components::ValueSum, zip32},
    rpc, token::{self, MaspDigitPos}, Namada,
};
use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};

use crate::{entities::Alias, sdk::namada::Sdk, steps::StepError};

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

pub async fn get_shielded_balance(
    sdk: &Sdk,
    source: Alias,
    retry_config: RetryFutureConfig<ExponentialBackoff, NoOnRetry>,
) -> Result<Option<token::Amount>, StepError> {
    shield_sync(sdk).await?;

    let client = sdk.namada.clone_client();
    let mut wallet = sdk.namada.wallet.write().await;
    let target_spending_key = wallet
        .find_spending_key(&source.name, None)
        .unwrap()
        .to_owned()
        .key;
    drop(wallet);

    let mut shielded_ctx = sdk.namada.shielded_mut().await;

    let viewing_key = zip32::ExtendedFullViewingKey::from(&target_spending_key.into())
        .fvk
        .vk;

    let balance = shielded_ctx
        .compute_shielded_balance(&viewing_key)
        .await
        .unwrap()
        .unwrap_or_else(|| ValueSum::zero());

    let mut amount = 0 as u64;
    for (asset_type, value) in balance.components() {
        let decoded = shielded_ctx.decode_asset_type(&client, *asset_type).await.unwrap();
        assert!(matches!(decoded.position, MaspDigitPos::Zero));
        amount += u64::try_from(*value).unwrap()
    }

    let token_balance = token::Amount::from_u64(amount);

    Ok(Some(token_balance))
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

pub async fn shield_sync(sdk: &Sdk) -> Result<(), StepError> {
    let wallet = sdk.namada.wallet.read().await;
    let vks: Vec<_> = sdk
        .namada
        .wallet()
        .await
        .get_viewing_keys()
        .values()
        .map(|evk| evk.map(|key| key.as_viewing_key()))
        .collect();
    drop(wallet);

    let mut shielded_ctx = sdk.namada.shielded_mut().await;

    let masp_client = LedgerMaspClient::new(sdk.namada.clone_client(), 100);
    let task_env = MaspLocalTaskEnv::new(4).map_err(|e| StepError::ShieldSync(e.to_string()))?;
    let shutdown_signal = install_shutdown_signal(true);

    let config = ShieldedSyncConfig::builder()
        .client(masp_client)
        .fetched_tracker(DevNullProgressBar)
        .scanned_tracker(DevNullProgressBar)
        .applied_tracker(DevNullProgressBar)
        .shutdown_signal(shutdown_signal)
        .wait_for_last_query_height(true)
        .build();

    shielded_ctx
        .sync(task_env, config, None, &[], &vks)
        .await
        .map_err(|e| StepError::ShieldedSync(e.to_string()))?;

    Ok(())
}
