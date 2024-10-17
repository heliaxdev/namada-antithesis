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
    io::{Client, NullIo},
    masp::{fs::FsShieldedUtils, ShieldedContext},
};
use namada_wallet::fs::{FsWalletUtils, FsWalletStorage};
use rand::RngCore;
use tempfile::{tempdir, TempDir};
use tendermint_rpc::{HttpClient, Url};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;
use std::env;
use std::path::{Path, PathBuf};
use file_lock::{FileLock, FileOptions};
use fs2::FileExt;
use std::fs::{File, OpenOptions};

#[tokio::main]
async fn main() {
    antithesis_init();
    
    let config = AppConfig::parse();

    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env()
        .unwrap()
        .add_directive("namada_chain_workload=debug".parse().unwrap())
        .add_directive("namada_sdk::rpc=debug".parse().unwrap());

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .compact()
        .init();


    tracing::info!("Using config: {:#?}", config);

    let step = match StepType::from_str(&config.step) {
        Ok(parsed_step) => parsed_step,
        Err(_) => {
            tracing::error!("Please provide an existing step type");
            std::process::exit(1);
        },
    };

    let base_dir = tempdir().unwrap().path().to_path_buf();
    println!("{:?}", base_dir);
    let base_dir = Path::new("/d/").join(".tmp");
    println!("{:?}", base_dir);


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
        let wallet_path = base_dir.join("wallet");

        let mut wallet = FsWalletUtils::new(wallet_path);
        // println!("wallet: {:#?}", wallet);
        let x = wallet.load();
        // println!("wallet after: {:#?}", wallet);

        // println!("store: {:#?}", wallet.store);

        // Setup shielded context storage
        let shielded_ctx_path = base_dir.join("masp");
        let shielded_ctx = ShieldedContext::new(FsShieldedUtils::new(shielded_ctx_path));
        println!("shielded_ctx: {:#?}", shielded_ctx);

        let io = NullIo;

        match Sdk::new(
            &config,
            &base_dir,
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

    let options = FileOptions::new()
                        .write(true)
                        .create(true)
                        .read(true)
                        .append(true);


    let file_path = "/state";
    let sl = String::from("/state_lock");
    let mut l = FLock::create(sl);
    l.lock();

    tracing::info!("State file locked");



    let seed = config.seed.unwrap_or(rand::thread_rng().next_u64());
    let mut state = State::read_from_file(&file_path, seed).unwrap_or(State::new(seed));

    tracing::info!("Using base dirsssss: {}", sdk.base_dir.as_path().display());
    tracing::info!("Using seeddddd: {}", seed);

    let workload_executor = WorkloadExecutor::new(
        vec![
            step
        ],
        vec![1.0],
    );

    tracing::info!("Starting initialization...");
    workload_executor.init(&sdk).await;
    tracing::info!("Done initialization!");

    let next_step = match workload_executor.next(&state) {
        Ok(step_type) => step_type,
        Err(_) => {
            l.unlock();
            tracing::error!("state file unlocked due to STEP validity exception");
            std::process::exit(1);
        }
    };

    tracing::info!("Next step is: {:?}...", next_step);
    let tasks = match workload_executor.build(next_step, &sdk, &mut state).await {
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
            // filelock.unlock();
            tracing::error!("state file unlocked due to BUILD exception");
            l.unlock();
            std::process::exit(1);
        }
    };
    tracing::info!("Built {:#?}...", next_step);
    // tracing::info!("tasks {:#?}...", tasks);
    // tracing::info!("staet {:#?}...", state);

    let checks = workload_executor
        .build_check(&sdk, tasks.clone(), &state)
        .await;
    tracing::info!("Built checks for {:?}", next_step);

    match workload_executor.execute(&sdk, tasks.clone()).await {
        Ok(secs) => {
            workload_executor.update_state(tasks, &mut state);
            tracing::info!("Execution took {}s...", secs);
        }
        Err(e) => {
            match e {
                namada_chain_workload::steps::StepError::Execution(_) => {
                    tracing::error!("Error {:?} -> {}", next_step, e.to_string());
                }
                _ => {
                    tracing::warn!("Warning {:?} -> {}", next_step, e.to_string());
                }
            }
            tracing::error!("state file unlocked due to EXECUTE exception");
            l.unlock();
            std::process::exit(1);
        }
    };
    // println!("{:?}", wallet);
    // println!("{:?}", shielded_ctx);


    if let Err(e) = workload_executor.checks(&sdk, checks.clone()).await {
        tracing::error!("Error {:?} (Check) -> {}", next_step, e.to_string());
    } else {
        if checks.is_empty() {
            tracing::info!("Checks are empty, skipping...");
        } else {
            tracing::info!("Checks were successful, updating state...");
        }
        tracing::info!("Done {:?}!", next_step);
    }
    state.write_to_file(&file_path).unwrap_or_else(|_| tracing::error!("Error: Could not save state, following validation invalid"));
    l.unlock();
}


struct FLock {
    path: String,
    file: File
}

impl FLock {
    fn create(path: String) -> FLock {
        let file = match OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)  // Create the file if it doesn't exist
        .open(&path) {
            Ok(file) => file,
            Err(err) => {
                tracing::error!("Could not open lock file");
                std::process::exit(1);
            }
        };
        FLock {path, file}
    }

    pub fn lock(&mut self) {
        self.file.lock_exclusive();
    }

    pub fn unlock(&mut self) {
        self.file.unlock();
        tracing::info!("state file unlocked");
    }
}

impl Drop for FLock {
    fn drop(&mut self) {
        self.file.unlock();
        tracing::error!("Drop: Unlocking lock file");
    }
}