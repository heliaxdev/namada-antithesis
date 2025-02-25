use std::{env, str::FromStr, time::Duration};

use antithesis_sdk::antithesis_init;
use clap::Parser;
use namada_chain_workload::code::Code;
use namada_chain_workload::config::AppConfig;
use namada_chain_workload::executor::{StepError, WorkloadExecutor};
use namada_chain_workload::sdk::namada::Sdk;
use namada_chain_workload::state::{State, StateError};
use namada_sdk::io::{Client, NullIo};
use namada_sdk::masp::fs::FsShieldedUtils;
use namada_sdk::masp::ShieldedContext;
use namada_wallet::fs::FsWalletUtils;
use serde_json::json;
use tendermint_rpc::{HttpClient, Url};
use tokio::time::sleep;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    let code = inner_main().await;

    code.output_logs();

    code.assert();

    if code.is_fatal() {
        std::process::exit(code.code());
    } else {
        // system state is as expected
        std::process::exit(0);
    }
}

async fn inner_main() -> Code {
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

    // just to report the workload version
    antithesis_sdk::assert_always!(
        true,
        "ID should be greater than 0",
        &json!({"commit_sha": env!("VERGEN_GIT_SHA")})
    );

    let (state, locked_file) = match State::load(config.id) {
        Ok(result) => result,
        Err(StateError::EmptyFile) => {
            tracing::warn!("State file is empty, creating new one");
            match State::create_new(config.id) {
                Ok(result) => result,
                Err(e) => return Code::StateFatal(e),
            }
        }
        Err(e) => return Code::StateFatal(e),
    };

    tracing::info!("Using base dir: {}", state.base_dir.as_path().display());

    let url = Url::from_str(&config.rpc).expect("invalid RPC address");
    tracing::debug!("Opening connection to {url}");
    let http_client = HttpClient::new(url).unwrap();

    // Wait for the first 2 blocks
    loop {
        match http_client.latest_block().await {
            Ok(block) if block.block.header.height.value() >= 2 => break,
            Ok(block) => tracing::info!(
                "Block height {}, waiting to be > 2...",
                block.block.header.height
            ),
            Err(e) => tracing::info!("No response from CometBFT, retrying... -> {e}"),
        }
        sleep(Duration::from_secs(5)).await;
    }

    let sdk = loop {
        match setup_sdk(&http_client, &state, &config).await {
            Ok(sdk) => break sdk,
            Err(_) => {
                tracing::info!("Setup SDK failed, retrying...");
                sleep(Duration::from_secs(2)).await;
            }
        }
    };

    let mut workload_executor = WorkloadExecutor::new(sdk, state);
    if let Err(e) = workload_executor.init().await {
        return Code::InitFatal(e);
    }

    let next_step = config.step_type;
    match workload_executor.is_valid(&next_step).await {
        Ok(true) => {}
        _ => {
            tracing::warn!(
                "Invalid step: {next_step} -> {:>?}",
                workload_executor.state()
            );
            return Code::InvalidStep(next_step);
        }
    }

    let init_block_height = workload_executor.fetch_current_block_height().await;

    tracing::info!("Step is: {next_step}...");
    let tasks = match workload_executor.build(&next_step).await {
        Ok(tasks) if tasks.is_empty() => {
            return Code::NoTask(next_step);
        }
        Ok(tasks) => tasks,
        Err(e) => {
            return Code::BuildFailure(next_step, e);
        }
    };
    tracing::info!("Built tasks for {next_step}");

    let checks = if config.no_check {
        vec![]
    } else {
        match workload_executor.build_check(&tasks).await {
            Ok(checks) => checks,
            Err(e) => return Code::BuildFailure(next_step, e),
        }
    };
    tracing::info!("Built checks for {next_step}");

    let execution_height = match workload_executor.execute(&tasks).await {
        Ok(height) => height,
        Err(e) if matches!(e, StepError::Execution(_)) => {
            workload_executor.update_failed_execution(&tasks); // remove fees
            return Code::ExecutionFailure(next_step, e);
        }
        Err(e) if matches!(e, StepError::Broadcast(_)) => {
            loop {
                let current_block_height = workload_executor.fetch_current_block_height().await;
                if current_block_height > init_block_height {
                    break;
                }
            }
            return Code::BroadcastFailure(next_step, e);
        }
        Err(StepError::EmptyBatch) => {
            return Code::EmptyBatch(next_step);
        }
        Err(e) => {
            return Code::OtherFailure(next_step, e);
        }
    };

    tracing::info!("Execution were successful, updating state...");
    if let Err(e) = workload_executor.post_execute(&tasks).await {
        return Code::Fatal(next_step, e);
    }

    let exit_code = match workload_executor.checks(checks, execution_height).await {
        Ok(_) => Code::Success(next_step),
        Err(e) => Code::Fatal(next_step, e),
    };

    tracing::info!("Statistics: {:>?}", workload_executor.state().stats);

    if let Err(e) = workload_executor.state().save(Some(locked_file)) {
        return Code::StateFatal(e);
    }

    exit_code
}

async fn setup_sdk(client: &HttpClient, state: &State, config: &AppConfig) -> Result<Sdk, String> {
    // Setup wallet storage
    let wallet_path = state.base_dir.join(format!("wallet-{}", config.id));
    let mut wallet = FsWalletUtils::new(wallet_path.clone());
    if wallet_path.join("wallet.toml").exists() {
        wallet.load().expect("Should be able to load the wallet");
    }

    // Setup shielded context storage
    let shielded_ctx_path = state.base_dir.join(format!("masp-{}", config.id));

    let mut shielded_ctx = ShieldedContext::new(FsShieldedUtils::new(shielded_ctx_path.clone()));
    if shielded_ctx_path.join("shielded.dat").exists() {
        shielded_ctx
            .load()
            .await
            .expect("Should be able to load shielded context");
    } else {
        shielded_ctx.save().await.unwrap();
    }

    Sdk::new(
        config,
        &state.base_dir,
        client.clone(),
        wallet,
        shielded_ctx,
        NullIo,
    )
    .await
}
