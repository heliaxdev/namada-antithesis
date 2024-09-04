use std::{str::FromStr, thread, time::Duration};

use antithesis_sdk::antithesis_init;
use clap::Parser;
use namada_chain_workload::{
    config::AppConfig,
    sdk::namada::Sdk,
    state::State,
    steps::{StepType, WorkloadExecutor},
};
use namada_sdk::{
    io::NullIo, masp::fs::FsShieldedUtils, queries::Client, wallet::fs::FsWalletUtils,
};
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
        &config,
        &base_dir,
        http_client.clone(),
        wallet,
        shielded_ctx,
        io,
    )
    .await;

    tracing::info!("Using base dir: {}", sdk.base_dir.as_path().display());

    let mut state = State::default();
    let workload_executor = WorkloadExecutor::new(
        vec![
            StepType::NewWalletKeyPair,
            StepType::FaucetTransfer,
            StepType::TransparentTransfer,
            StepType::Bond,
        ],
        vec![2.0, 3.0, 6.0, 6.0],
    );

    tracing::info!("Starting initialization...");
    workload_executor.init(&sdk).await;
    tracing::info!("Done initialization!");

    loop {
        let next_step = workload_executor.next(&state);
        tracing::info!("Next step is: {:?}...", next_step);
        let tasks = match workload_executor.build(next_step, &sdk, &state).await {
            Ok(tasks) => tasks,
            Err(e) => {
                match e {
                    namada_chain_workload::steps::StepError::Execution(_) => {
                        tracing::error!("Error {:?} -> {}", next_step, e.to_string());
                    }
                    _ => {
                        tracing::warn!("Warning {:?} -> {}", next_step, e.to_string());
                    }
                }
                continue;
            }
        };
        tracing::info!("Built {:?}...", next_step);

        let checks = workload_executor.build_check(&sdk, tasks.clone()).await;
        tracing::info!("Built checks for {:?}", next_step);

        match workload_executor.execute(&sdk, tasks.clone()).await {
            Ok(secs) => tracing::info!("Execution took {}s...", secs),
            Err(e) => {
                match e {
                    namada_chain_workload::steps::StepError::Execution(_) => {
                        tracing::error!("Error {:?} -> {}", next_step, e.to_string());
                    }
                    _ => {
                        tracing::warn!("Warning {:?} -> {}", next_step, e.to_string());
                    }
                }
                continue;
            }
        };

        if let Err(e) = workload_executor.checks(&sdk, checks.clone()).await {
            tracing::error!("Error {:?} (Check) -> {}", next_step, e.to_string());
            continue;
        } else {
            if checks.is_empty() {
                tracing::info!("Checks are empty, skipping...");
            } else {
                tracing::info!("Checks were successful, updating state...");
            }
            workload_executor.update_state(tasks, &mut state);
            tracing::info!("Done {:?}!", next_step);
        }
        println!(" ")
    }
}
