use std::{str::FromStr, thread, time::Duration};

use antithesis_sdk::antithesis_init;
use clap::Parser;
use namada_chain_check::{
    checks::{
        epoch::EpochCheck, height::HeightCheck, inflation::InflationCheck,
        masp_indexer::MaspIndexerHeightCheck, status::StatusCheck, voting_power::VotingPowerCheck,
        DoCheck,
    },
    config::AppConfig,
    sdk::namada::Sdk,
    state::State,
};
use namada_sdk::{io::NullIo, masp::fs::FsShieldedUtils, wallet::fs::FsWalletUtils};
use serde_json::json;
use tempfile::tempdir;
use tendermint_rpc::{Client, HttpClient, Url};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    antithesis_init();

    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env()
        .unwrap();

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .compact()
        .without_time()
        .with_ansi(false)
        .init();

    let config = AppConfig::parse();
    tracing::info!("Using config: {:#?}", config);

    let base_dir = tempdir().unwrap().path().to_path_buf();

    let url = Url::from_str(&config.rpc).expect("invalid RPC address");
    let http_client = HttpClient::new(url).unwrap();

    // Setup wallet storage
    let wallet_path = base_dir.join("wallet");
    let wallet = FsWalletUtils::new(wallet_path);

    // Setup shielded context storage
    let shielded_ctx_path = base_dir.join("masp");
    let shielded_ctx = FsShieldedUtils::new(shielded_ctx_path);

    let io = NullIo;

    let mut state = State::from_height(2);

    // Wait for the first 2 blocks
    loop {
        let latest_blocked = http_client.latest_block().await;
        if let Ok(block) = latest_blocked {
            if block.block.header.height.value() >= 2 {
                break;
            } else {
                tracing::info!(
                    "block height {}, waiting to be > 2...",
                    block.block.header.height
                );
                thread::sleep(Duration::from_secs(2));
            }
        } else {
            tracing::info!("no response from cometbft, retrying in 2...");
            thread::sleep(Duration::from_secs(2));
        }
    }

    loop {
        let client = reqwest::Client::new();
        if let Ok(res) = client
            .get(format!("{}/health", config.masp_indexer_url))
            .send()
            .await
        {
            if res.status().is_success() {
                break;
            } else {
                tracing::info!("waiting for masp-indexer to be responsive...",);
                thread::sleep(Duration::from_secs(2));
            }
        } else {
            tracing::info!("no response from masp-indexer, retrying in 2...");
            thread::sleep(Duration::from_secs(2));
        }
    }

    let sdk = Sdk::new(
        &base_dir,
        http_client.clone(),
        wallet,
        shielded_ctx,
        io,
        config.masp_indexer_url,
    )
    .await;

    loop {
        let now = chrono::offset::Utc::now();

        let vp_check_res = VotingPowerCheck::do_check(&sdk, &mut state, now).await;
        is_succesful(VotingPowerCheck::to_string(), vp_check_res);

        let height_check_res = HeightCheck::do_check(&sdk, &mut state, now).await;
        is_succesful(HeightCheck::to_string(), height_check_res);

        let epoch_check_res = EpochCheck::do_check(&sdk, &mut state, now).await;
        is_succesful(EpochCheck::to_string(), epoch_check_res);

        let inflation_check_res = InflationCheck::do_check(&sdk, &mut state, now).await;
        is_succesful(InflationCheck::to_string(), inflation_check_res);

        let status_check_res = StatusCheck::do_check(&sdk, &mut state, now).await;
        is_succesful(StatusCheck::to_string(), status_check_res);

        let masp_indexer_check_res = MaspIndexerHeightCheck::do_check(&sdk, &mut state, now).await;
        is_succesful(MaspIndexerHeightCheck::to_string(), masp_indexer_check_res);

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

fn is_succesful(check_name: String, res: Result<(), String>) {
    if let Err(e) = res.clone() {
        let is_timeout = e.to_lowercase().contains("timed out");
        let is_connection_closed = e.to_lowercase().contains("connection closed before");
        if is_timeout {
            tracing::warn!("Check {} has timedout", check_name);
            return;
        }
        if is_connection_closed {
            tracing::warn!(
                "Check {} has failed due to connection closed before message completed",
                check_name
            );
            return;
        }

        match check_name.as_ref() {
            "HeightCheck" => {
                antithesis_sdk::assert_always!(
                    res.is_ok(),
                    "Block height increased",
                    &json!({ "details": e })
                );
            }
            "EpochCheck" => {
                antithesis_sdk::assert_always!(
                    res.is_ok(),
                    "Epoch increased",
                    &json!({ "details": e })
                );
            }
            "InflationCheck" => {
                antithesis_sdk::assert_always!(
                    res.is_ok(),
                    "Inflation increased",
                    &json!({ "details": e })
                );
            }
            "MaspIndexerHeightCheck" => {
                antithesis_sdk::assert_sometimes!(
                    res.is_ok(),
                    "Masp indexer block height increased",
                    &json!({ "details": e })
                );
            }
            _ => {
                tracing::warn!("Check {} assertion not found (err)...", check_name);
            }
        }
        tracing::error!("{}", format!("Error! {}: {}", check_name, e));
    } else {
        match check_name.as_ref() {
            "HeightCheck" => {
                antithesis_sdk::assert_always!(res.is_ok(), "Block height increased", &json!({}));
            }
            "EpochCheck" => {
                antithesis_sdk::assert_always!(res.is_ok(), "Epoch increased", &json!({}));
            }
            "InflationCheck" => {
                antithesis_sdk::assert_always!(res.is_ok(), "Inflation increased", &json!({}));
            }
            "MaspIndexerHeightCheck" => {
                antithesis_sdk::assert_sometimes!(
                    res.is_ok(),
                    "Masp indexer block height increased",
                    &json!({})
                );
            }
            _ => {
                tracing::warn!("Check {} assertion not found...", check_name);
            }
        }
        tracing::debug!("{}", format!("Check {} was successful.", check_name));
    }
}
