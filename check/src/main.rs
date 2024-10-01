use std::{str::FromStr, thread, time::Duration};

use antithesis_sdk::antithesis_init;
use clap::Parser;
use namada_chain_check::{
    checks::{
        epoch::EpochCheck, height::HeightCheck, inflation::InflationCheck, masp::MaspIndexerCheck,
        status::StatusCheck, voting_power::VotingPowerCheck, DoCheck,
    },
    config::AppConfig,
    sdk::namada::Sdk,
    state::State,
};
use namada_sdk::{
    io::NullIo, masp::fs::FsShieldedUtils, queries::Client, wallet::fs::FsWalletUtils,
};
use serde_json::json;
use tempfile::tempdir;
use tendermint_rpc::{HttpClient, Url};

#[tokio::main]
async fn main() {
    antithesis_init();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
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
            }
        } else {
            tracing::info!("no response from cometbft, retrying in 5...");
            thread::sleep(Duration::from_secs(5));
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

        let height_check_res = HeightCheck::do_check(&sdk, &mut state, now).await;
        is_succesful(HeightCheck::to_string(), height_check_res);

        let epoch_check_res = EpochCheck::do_check(&sdk, &mut state, now).await;
        is_succesful(EpochCheck::to_string(), epoch_check_res);

        let inflation_check_res = InflationCheck::do_check(&sdk, &mut state, now).await;
        is_succesful(InflationCheck::to_string(), inflation_check_res);

        let status_check_res = StatusCheck::do_check(&sdk, &mut state, now).await;
        is_succesful(StatusCheck::to_string(), status_check_res);

        let vp_check_res = VotingPowerCheck::do_check(&sdk, &mut state, now).await;
        is_succesful(VotingPowerCheck::to_string(), vp_check_res);

        let masp_indexer_check = MaspIndexerCheck::do_check(&sdk, &mut state, now).await;
        is_succesful(MaspIndexerCheck::to_string(), masp_indexer_check);

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
                    "height_check",
                    &json!({ "details": e })
                );
            }
            "EpochCheck" => {
                antithesis_sdk::assert_always!(
                    res.is_ok(),
                    "epoch_check",
                    &json!({ "details": e })
                );
            }
            "InflationCheck" => {
                antithesis_sdk::assert_always!(
                    res.is_ok(),
                    "inflation_check",
                    &json!({ "details": e })
                );
            }
            "MaspIndexerCheck" => {
                antithesis_sdk::assert_sometimes!(
                    res.is_ok(),
                    "masp_indexer_check",
                    &json!({ "details": e })
                );
            }
            _ => (),
        }
        tracing::error!("{}", format!("Error! {}: {}", check_name, e));
    } else {
        match check_name.as_ref() {
            "HeightCheck" => {
                antithesis_sdk::assert_always!(res.is_ok(), "height_check", &json!({}));
            }
            "EpochCheck" => {
                antithesis_sdk::assert_always!(res.is_ok(), "epoch_check", &json!({}));
            }
            "InflationCheck" => {
                antithesis_sdk::assert_always!(res.is_ok(), "inflation_check", &json!({}));
            }
            "MaspIndexerCheck" => {
                antithesis_sdk::assert_sometimes!(res.is_ok(), "masp_indexer_check", &json!({}));
            }
            _ => (),
        }
        tracing::debug!("{}", format!("Check {} was successful.", check_name));
    }
}
