use std::{env, fs::File, str::FromStr, thread, time::Duration};

use antithesis_sdk::{antithesis_init, lifecycle};
use clap::Parser;
use fs2::FileExt;
use namada_chain_workload::{
    config::AppConfig,
    sdk::namada::Sdk,
    state::State,
    steps::{StepType, WorkloadExecutor},
};
use namada_sdk::{
    io::{Client, NullIo},
    masp::{fs::FsShieldedUtils, ShieldedContext},
};
use namada_wallet::fs::FsWalletUtils;
use serde_json::json;
use tendermint_rpc::{HttpClient, Url};
use tokio::time::sleep;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    let (exit_code, step_type) = inner_main().await;

    match step_type {
        StepType::NewWalletKeyPair => {
            antithesis_sdk::assert_always!(
                exit_code != 1,
                "Done executing NewWalletKeyPair",
                &json!({"outcome":exit_code})
            );
        }
        StepType::FaucetTransfer => {
            antithesis_sdk::assert_always!(
                exit_code != 1,
                "Done executing FaucetTransfer",
                &json!({"outcome":exit_code})
            );
        }
        StepType::TransparentTransfer => {
            antithesis_sdk::assert_always!(
                exit_code != 1,
                "Done executing TransparentTransfer",
                &json!({"outcome":exit_code})
            );
        }
        StepType::Bond => {
            antithesis_sdk::assert_always!(
                exit_code != 1,
                "Done executing Bond",
                &json!({"outcome":exit_code})
            );
        }
        StepType::InitAccount => {
            antithesis_sdk::assert_always!(
                exit_code != 1,
                "Done executing InitAccount",
                &json!({"outcome":exit_code})
            );
        }
        StepType::Redelegate => {
            antithesis_sdk::assert_always!(
                exit_code != 1,
                "Done executing Redelegate",
                &json!({"outcome":exit_code})
            );
        }
        StepType::Unbond => {
            antithesis_sdk::assert_always!(
                exit_code != 1,
                "Done executing Unbond",
                &json!({"outcome":exit_code})
            );
        }
        StepType::ClaimRewards => {
            antithesis_sdk::assert_always!(
                exit_code != 1,
                "Done executing ClaimRewards",
                &json!({"outcome":exit_code})
            );
        }
        StepType::BatchBond => {
            antithesis_sdk::assert_always!(
                exit_code != 1,
                "Done executing BatchBond",
                &json!({"outcome":exit_code})
            );
        }
        StepType::BatchRandom => {
            antithesis_sdk::assert_always!(
                exit_code != 1,
                "Done executing BatchRandom",
                &json!({"outcome":exit_code})
            );
        }
        StepType::Shielding => {
            antithesis_sdk::assert_always!(
                exit_code != 1,
                "Done executing Shielding",
                &json!({"outcome":exit_code})
            );
        }
        StepType::Shielded => {
            antithesis_sdk::assert_always!(
                exit_code != 1,
                "Done executing Shielded",
                &json!({"outcome":exit_code})
            );
        }
        StepType::Unshielding => {
            antithesis_sdk::assert_always!(
                exit_code != 1,
                "Done executing Unshielding",
                &json!({"outcome":exit_code})
            );
        }
        StepType::BecomeValidator => {
            antithesis_sdk::assert_always!(
                exit_code != 1,
                "Done executing BecomeValidator",
                &json!({"outcome":exit_code})
            );
        }
        StepType::ChangeMetadata => {
            antithesis_sdk::assert_always!(
                exit_code != 1,
                "Done executing ChangeMetadata",
                &json!({"outcome":exit_code})
            );
        }
    }

    std::process::exit(exit_code);
}

async fn inner_main() -> (i32, StepType) {
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

    rlimit::increase_nofile_limit(10240).unwrap();
    rlimit::increase_nofile_limit(u64::MAX).unwrap();

    let config = AppConfig::parse();
    tracing::info!("Using config: {:#?}", config);
    tracing::info!("Sha commit: {}", env!("VERGEN_GIT_SHA").to_string());

    tracing::info!("Trying to get the lock...");
    let path = env::current_dir()
        .unwrap()
        .join(format!("state-{}.json", config.id));

    let file = File::open(&path).expect(&format!("Could not open {:?}", path));

    file.lock_exclusive().unwrap();
    tracing::info!("State locked.");

    let mut state = State::from_file(config.id, config.seed);

    tracing::info!("Using base dir: {}", state.base_dir.as_path().display());
    tracing::info!("Using seed: {}", state.seed);
    tracing::info!("With checks: {}", !config.no_check);

    let url = Url::from_str(&config.rpc).expect("invalid RPC address");
    tracing::info!("Opening connection agains {:?}", url);
    let http_client = HttpClient::new(url).unwrap();

    // Wait for the first 2 blocks
    loop {
        let latest_blocked = http_client.latest_block().await;
        match latest_blocked {
            Ok(block) => {
                if block.block.header.height.value() >= 2 {
                    break;
                } else {
                    tracing::info!(
                        "block height {}, waiting to be > 2...",
                        block.block.header.height
                    );
                }
            }
            Err(err) => {
                tracing::info!("no response from cometbft, retrying in 5... {}", err);
                thread::sleep(Duration::from_secs(5));
            }
        }
    }

    let sdk = loop {
        // Setup wallet storage
        let wallet_path = state.base_dir.join(format!("wallet-{}", config.id));
        let mut wallet = FsWalletUtils::new(wallet_path.clone());
        if wallet_path.join("wallet.toml").exists() {
            wallet.load().expect("Should be able to load the wallet");
        }

        // Setup shielded context storage
        let shielded_ctx_path = state.base_dir.join(format!("masp-{}", config.id));

        let mut shielded_ctx =
            ShieldedContext::new(FsShieldedUtils::new(shielded_ctx_path.clone()));
        if shielded_ctx_path.join("shielded.dat").exists() {
            shielded_ctx
                .load()
                .await
                .expect("Should be able to load shielded context");
        } else {
            shielded_ctx.save().await.unwrap();
        }

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

    let next_step = config.step_type; // bond
    if !workload_executor.is_valid(&next_step, &state) {
        tracing::warn!("Invalid step: {} -> {:>?}", next_step, state);
        return (0_i32, config.step_type);
    }

    let init_block_height = fetch_current_block_height(&sdk).await;

    tracing::info!("Step is: {:?}...", next_step);
    let tasks = match workload_executor.build(next_step, &sdk, &mut state).await {
        Ok(tasks) if tasks.len() == 0 => {
            tracing::info!("Couldn't build {:?}, skipping...", next_step);
            return (6_i32, config.step_type);
        }
        Ok(tasks) => tasks,
        Err(e) => {
            tracing::warn!("Warning build {:?} -> {}", next_step, e.to_string());
            return (7_i32, config.step_type);
        }
    };
    tracing::info!(
        "Built {:?} -> {:?}",
        next_step,
        tasks
            .iter()
            .map(|task| task.to_string())
            .collect::<Vec<String>>()
    );

    let checks = workload_executor
        .build_check(&sdk, tasks.clone(), &state, config.no_check)
        .await;
    tracing::info!("Built checks for {:?}", next_step);

    let execution_height = match workload_executor.execute(&sdk, tasks.clone()).await {
        Ok(result) => {
            let total_time_takes: u64 = result.iter().map(|execution| execution.time_taken).sum();
            tracing::info!("Execution took {}s...", total_time_takes);
            result
                .iter()
                .filter_map(|execution| execution.execution_height)
                .max()
        }
        Err(e) => match e {
            namada_chain_workload::steps::StepError::Execution(_) => {
                tracing::error!("Error executing{:?} -> {}", next_step, e.to_string());
                state.update_failed_execution(&tasks); // remove fees
                return (3_i32, config.step_type);
            }
            namada_chain_workload::steps::StepError::Broadcast(e) => {
                tracing::info!(
                    "Broadcasting error {:?} -> {}, waiting for next block",
                    next_step,
                    e.to_string()
                );
                loop {
                    let current_block_height = fetch_current_block_height(&sdk).await;
                    if current_block_height > init_block_height {
                        break;
                    }
                }
                return (4_i32, config.step_type);
            }
            namada_chain_workload::steps::StepError::EmptyBatch => {
                tracing::error!(
                    "Error building an empty batch{:?} -> {}",
                    next_step,
                    e.to_string()
                );
                return (8_i32, config.step_type);
            }
            _ => {
                tracing::warn!("Warning executing {:?} -> {}", next_step, e.to_string());
                return (5_i32, config.step_type);
            }
        },
    };

    let exit_code = if let Err(e) = workload_executor
        .checks(&sdk, checks.clone(), execution_height)
        .await
    {
        tracing::error!("Error final checks {:?} -> {}", next_step, e.to_string());
        1_i32
    } else if checks.is_empty() {
        workload_executor.update_state(tasks, &mut state);
        tracing::info!("Checks are empty, skipping checks and upadating state...");
        0_i32
    } else {
        workload_executor.update_state(tasks, &mut state);
        tracing::info!("Checks were successful, updating state...");
        0_i32
    };

    tracing::info!("Statistics: {:>?}", state.stats);

    state.serialize_to_file();
    let path = env::current_dir()
        .unwrap()
        .join(format!("state-{}.json", config.id));
    let file = File::open(path).unwrap();
    file.unlock().unwrap();
    tracing::info!("Done {:?}!", next_step);

    (exit_code, config.step_type)
}

async fn fetch_current_block_height(sdk: &Sdk) -> u64 {
    let client = sdk.namada.clone_client();
    loop {
        let latest_block = client.latest_block().await;
        if let Ok(block) = latest_block {
            return block.block.header.height.into();
        }
        sleep(Duration::from_secs_f64(1.0f64)).await
    }
}
