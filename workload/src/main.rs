use std::{env, fs::File, str::FromStr, thread, time::Duration};

use antithesis_sdk::antithesis_init;
use clap::Parser;
use fs2::FileExt;
use namada_chain_workload::{
    config::AppConfig, sdk::namada::Sdk, state::State, steps::WorkloadExecutor,
};
use namada_sdk::{
    io::{Client, NullIo},
    masp::{fs::FsShieldedUtils, ShieldedContext},
};
use namada_wallet::fs::FsWalletUtils;
use tendermint_rpc::{HttpClient, Url};
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
    tracing::info!("Sha commit: {}", env!("VERGEN_GIT_SHA").to_string());

    tracing::info!("Trying to get the lock...");
    let path = env::current_dir()
        .unwrap()
        .join(format!("state-{}.json", config.seed));
    let file = File::open(&path).unwrap();
    file.lock_exclusive().unwrap();
    tracing::info!("State locked.");

    let mut state = State::from_file(config.seed);

    tracing::info!("Using base dir: {}", state.base_dir.as_path().display());
    tracing::info!("Using seed: {}", state.seed);

    let url = Url::from_str(&config.rpc).expect("invalid RPC address");
    let http_client = HttpClient::new(url).unwrap();

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

    let sdk = loop {
        // Setup wallet storage
        let wallet_path = state.base_dir.join("wallet");
        let mut wallet = FsWalletUtils::new(wallet_path.clone());
        if wallet_path.join("wallet.toml").exists() {
            wallet.load().expect("Should be able to load the wallet;");
        }

        // Setup shielded context storage
        let shielded_ctx_path = state.base_dir.join("masp");
        let shielded_ctx = ShieldedContext::new(FsShieldedUtils::new(shielded_ctx_path));

        let io = NullIo;

        match Sdk::new(
            &config,
            &state.base_dir,
            http_client.clone(),
            wallet,
            shielded_ctx,
            io,
        )
        .await
        {
            Ok(sdk) => break sdk,
            Err(_) => std::thread::sleep(Duration::from_secs(2)),
        };
    };

    let workload_executor = WorkloadExecutor::new();

    tracing::info!("Starting initialization...");
    workload_executor.init(&sdk).await;
    tracing::info!("Done initialization!");

    let next_step = config.step_type;
    if !workload_executor.is_valid(&next_step, &state) {
        tracing::info!("Invalid step: {}", next_step);
        return;
    }

    tracing::info!("Step is: {:?}...", next_step);
    let tasks = match workload_executor.build(next_step, &sdk, &mut state).await {
        Ok(tasks) => tasks,
        Err(e) => {
            match e {
                namada_chain_workload::steps::StepError::Execution(_) => {
                    tracing::error!("Error build {:?} -> {}", next_step, e.to_string());
                }
                _ => {
                    tracing::warn!("Warning build {:?} -> {}", next_step, e.to_string());
                }
            }
            state.serialize_to_file();
            return;
        }
    };
    tracing::info!("Built {:?}...", next_step);

    let checks = workload_executor
        .build_check(&sdk, tasks.clone(), &state)
        .await;
    tracing::info!("Built checks for {:?}", next_step);

    let execution_height = match workload_executor.execute(&sdk, tasks.clone()).await {
        Ok(result) => {
            tracing::info!("Execution took {}s...", result.time_taken);
            result.execution_height
        }
        Err(e) => {
            match e {
                namada_chain_workload::steps::StepError::Execution(_) => {
                    tracing::error!("Error executing{:?} -> {}", next_step, e.to_string());
                }
                _ => {
                    tracing::warn!("Warning executing {:?} -> {}", next_step, e.to_string());
                }
            }
            state.serialize_to_file();
            return;
        }
    };

    if let Err(e) = workload_executor
        .checks(&sdk, checks.clone(), execution_height)
        .await
    {
        tracing::error!("Error final checks {:?} -> {}", next_step, e.to_string());
    } else {
        if checks.is_empty() {
            tracing::info!("Checks are empty, skipping...");
        } else {
            workload_executor.update_state(tasks, &mut state);
            tracing::info!("Checks were successful, updating state...");
        }
    }

    state.serialize_to_file();
    let path = env::current_dir()
        .unwrap()
        .join(format!("state-{}.json", config.seed));
    let file = File::open(&path).unwrap();
    file.unlock().unwrap();
    tracing::info!("Done {:?}!", next_step);
}