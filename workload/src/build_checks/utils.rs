use std::{
    collections::BTreeMap,
    str::FromStr,
    thread,
    time::{Duration, Instant},
};

use namada_sdk::{
    address::Address,
    control_flow::install_shutdown_signal,
    io::{DevNullProgressBar, Io, NullIo},
    masp::{
        shielded_wallet::ShieldedApi, IndexerMaspClient, LedgerMaspClient, MaspLocalTaskEnv,
        ShieldedSyncConfig,
    },
    masp_primitives::{transaction::components::ValueSum, zip32},
    rpc,
    token::{self, DenominatedAmount, MaspDigitPos, MaspEpoch},
    Namada,
};
use reqwest::Url;
use serde_json::json;
use tokio::time::sleep;
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
    height: Option<u64>,
    with_indexer: bool,
) -> Result<Option<token::Amount>, StepError> {
    let (res, error) = match shield_sync(sdk, height, with_indexer).await {
        Ok(_) => (true, "".to_string()),
        Err(e) => (false, e.to_string()),
    };

    tracing::warn!("First shieldsync result: {}, err: {}", res, error);

    if with_indexer {
        antithesis_sdk::assert_sometimes!(
            res,
            "Shieldsync (indexer) was successful.",
            &json!({
                "source": source,
                "error": error
            })
        );
    } else {
        antithesis_sdk::assert_always_or_unreachable!(
            res,
            "Shieldsync (node) was successful.",
            &json!({
                "source": source,
                "error": error
            })
        );
    }

    if with_indexer && !res {
        let (res, error) = match shield_sync(sdk, height, false).await {
            Ok(_) => (true, "".to_string()),
            Err(e) => (false, e.to_string()),
        };

        tracing::warn!("Second shieldsync result: {}/{}", res, error);

        antithesis_sdk::assert_always_or_unreachable!(
            res,
            "Shieldsync (node) was successful.",
            &json!({
                "source": source,
                "error": error
            })
        );
    }

    let client = sdk.namada.clone_client();

    let masp_epoch = rpc::query_epoch(&client)
        .await
        .map(|epoch| MaspEpoch::try_from_epoch(epoch, 2).unwrap())
        .map_err(|e| StepError::ShieldSync(e.to_string()))?;
    let native_token = rpc::query_native_token(&client)
        .await
        .map_err(|e| StepError::ShieldSync(e.to_string()))?;

    let mut wallet = sdk.namada.wallet.write().await;
    let spending_key = format!(
        "{}-spending-key",
        source.name.strip_suffix("-payment-address").unwrap()
    );
    let target_spending_key = wallet
        .find_spending_key(&spending_key, None)
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
        .map_err(|e| StepError::ShieldSync(e.to_string()))?;

    let balance = if let Some(balance) = balance {
        balance
    } else {
        return Ok(Some(token::Amount::from_u64(0)));
    };

    let total_balance = shielded_ctx
        .decode_combine_sum_to_epoch(&client, balance, masp_epoch)
        .await
        .0
        .get(&native_token);

    Ok(Some(total_balance.into()))
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

pub async fn shield_sync(
    sdk: &Sdk,
    height: Option<u64>,
    with_indexer: bool,
) -> Result<(), StepError> {
    let now = Instant::now();
    tracing::info!("Started shieldsync (using indexer: {})...", with_indexer);

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

    let mut max_retries = 3;
    if with_indexer {
        loop {
            let masp_client = IndexerMaspClient::new(
                reqwest::Client::new(),
                Url::parse(&sdk.masp_indexer_url).unwrap(),
                true,
                10,
            );
            let task_env =
                MaspLocalTaskEnv::new(4).map_err(|e| StepError::ShieldSync(e.to_string()))?;
            let shutdown_signal = install_shutdown_signal(true);

            let config = ShieldedSyncConfig::builder()
                .client(masp_client)
                .fetched_tracker(DevNullProgressBar)
                .scanned_tracker(DevNullProgressBar)
                .applied_tracker(DevNullProgressBar)
                .shutdown_signal(shutdown_signal);

            let config = if height.is_some() {
                config.wait_for_last_query_height(true).build()
            } else {
                config.build()
            };

            let height = height.map(|h| h.into());

            tracing::info!("Using height with shieldsync: {:?}", height);

            let res = shielded_ctx.sync(task_env, config, height, &[], &vks).await;
            if res.is_err() {
                tracing::info!("Retry (masp) shieldsyncing ({}/3)...", max_retries);
                if max_retries == 0 {
                    res.map_err(|e| StepError::ShieldedSync(e.to_string()))?
                }
                max_retries -= 1;
                sleep(Duration::from_secs(2)).await
            } else {
                break;
            }
        }
    } else {
        loop {
            let masp_client =
                LedgerMaspClient::new(sdk.namada.clone_client(), 10, Duration::from_secs(1));
            let task_env =
                MaspLocalTaskEnv::new(4).map_err(|e| StepError::ShieldSync(e.to_string()))?;
            let shutdown_signal = install_shutdown_signal(true);

            let config = ShieldedSyncConfig::builder()
                .client(masp_client)
                .fetched_tracker(DevNullProgressBar)
                .scanned_tracker(DevNullProgressBar)
                .applied_tracker(DevNullProgressBar)
                .shutdown_signal(shutdown_signal);

            let config = if height.is_some() {
                config.wait_for_last_query_height(true).build()
            } else {
                config.build()
            };

            let height = height.map(|h| h.into());

            tracing::info!("Using height with shieldsync: {:?}", height);

            let res = shielded_ctx.sync(task_env, config, height, &[], &vks).await;
            if res.is_err() {
                tracing::info!("Retry (node) shieldsyncing ({}/3)...", max_retries);
                if max_retries == 0 {
                    res.map_err(|e| StepError::ShieldedSync(e.to_string()))?
                }
                max_retries -= 1;
                sleep(Duration::from_secs(2)).await
            } else {
                break;
            }
        }
    };

    shielded_ctx
        .save()
        .await
        .map_err(|e| StepError::ShieldedSync(e.to_string()))?;

    tracing::info!(
        "Done shieldsync (took {}s, with indexer: {})!",
        now.elapsed().as_secs(),
        with_indexer
    );

    Ok(())
}
