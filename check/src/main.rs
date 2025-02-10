use std::{str::FromStr, thread, time::Duration};

use antithesis_sdk::antithesis_init;
use clap::Parser;
use namada_chain_check::{checks::try_checks, config::AppConfig, sdk::namada::Sdk, state::State};
use namada_sdk::{io::NullIo, masp::fs::FsShieldedUtils, wallet::fs::FsWalletUtils};
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
        try_checks(&sdk, &mut state).await;

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
