use std::{str::FromStr, thread, time::Duration};

use antithesis_sdk::antithesis_init;
use clap::Parser;
use namada_chain_check::{
    checks::{epoch::EpochCheck, height::HeightCheck, inflation::InflationCheck, DoCheck},
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

    let config = AppConfig::parse();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

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

    let sdk = Sdk::new(&base_dir, http_client.clone(), wallet, shielded_ctx, io).await;

    let mut state = State::from_height(3);

    // Wait for the first 1 blocks
    loop {
        let latest_blocked = http_client.latest_block().await;
        if let Ok(block) = latest_blocked {
            if block.block.header.height.value() > 1 {
                break;
            }
        } else {
            thread::sleep(Duration::from_secs(10));
        }
    }

    loop {
        let height_check_res = HeightCheck::do_check(&sdk, &mut state).await;
        is_succesful(HeightCheck::to_string(), height_check_res);

        let epoch_check_res = EpochCheck::do_check(&sdk, &mut state).await;
        is_succesful(EpochCheck::to_string(), epoch_check_res);

        let inflation_check_res = InflationCheck::do_check(&sdk, &mut state).await;
        is_succesful(InflationCheck::to_string(), inflation_check_res);

        tokio::time::sleep(Duration::from_secs(config.timeout.unwrap_or(30))).await;
    }
}

fn is_succesful(check_name: String, res: Result<(), String>) {
    if let Err(e) = res.clone() {
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
            _ => (),
        }
        tracing::error!("{}", format!("{}: {}", check_name, e));
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
            _ => (),
        }
        tracing::info!("{}", format!("{} ok", check_name));
    }
}
