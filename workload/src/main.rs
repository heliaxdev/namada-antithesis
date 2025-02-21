use std::{env, str::FromStr, thread, time::Duration};

use antithesis_sdk::antithesis_init;
use clap::Parser;
use namada_chain_workload::{
    assert_step,
    config::AppConfig,
    sdk::namada::Sdk,
    state::{State, StateError},
    steps::{StepError, StepType, WorkloadExecutor},
};
use namada_sdk::{
    io::{Client, NullIo},
    masp::{fs::FsShieldedUtils, ShieldedContext},
    rpc,
};
use namada_wallet::fs::FsWalletUtils;
use serde_json::json;
use tendermint_rpc::{HttpClient, Url};
use tokio::time::sleep;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

enum Code {
    Success(StepType),
    Fatal(StepType, StepError),
    ExecutionFailure(StepType, StepError),
    BroadcastFailure(StepType, StepError),
    OtherFailure(StepType, StepError),
    BuildFailure(StepType, StepError),
    InvalidStep(StepType),
    NoTask(StepType),
    EmptyBatch(StepType),
    StateFatal(StateError),
}

impl Code {
    fn code(&self) -> i32 {
        match self {
            Code::Success(_) | Code::InvalidStep(_) => 0,
            Code::Fatal(_, _) => 1,
            Code::BuildFailure(_, _) => 2,
            Code::ExecutionFailure(_, _) => 3,
            Code::BroadcastFailure(_, _) => 4,
            Code::OtherFailure(_, _) => 5,
            Code::NoTask(_) => 6,
            Code::EmptyBatch(_) => 7,
            Code::StateFatal(_) => 8,
        }
    }

    fn step_type(&self) -> Option<StepType> {
        match self {
            Code::Success(st) => Some(*st),
            Code::Fatal(st, _) => Some(*st),
            Code::ExecutionFailure(st, _) => Some(*st),
            Code::BroadcastFailure(st, _) => Some(*st),
            Code::OtherFailure(st, _) => Some(*st),
            Code::BuildFailure(st, _) => Some(*st),
            Code::InvalidStep(st) => Some(*st),
            Code::NoTask(st) => Some(*st),
            Code::EmptyBatch(st) => Some(*st),
            Code::StateFatal(_) => None,
        }
    }

    fn output_logs(&self) {
        match self {
            Code::Success(step_type) => tracing::info!("Done {step_type} successfully!"),
            Code::Fatal(step_type, reason) => {
                tracing::error!("State check error for {step_type} -> {reason}")
            }
            Code::ExecutionFailure(step_type, reason) => {
                tracing::error!("Transaction execution failure for {step_type} -> {reason}")
            }
            Code::BroadcastFailure(step_type, reason) => tracing::info!(
                "Transaction broadcast failure for {step_type} -> {reason}, waiting for next block"
            ),
            Code::OtherFailure(step_type, reason) => {
                tracing::warn!("Failure for {step_type} -> {reason}")
            }
            Code::InvalidStep(step_type) => {
                tracing::warn!("Invalid step for {step_type}, skipping...")
            }
            Code::NoTask(step_type) => tracing::info!("No task for {step_type}, skipping..."),
            Code::BuildFailure(step_type, reason) => {
                tracing::warn!("Build failure for {step_type} -> {reason}")
            }
            Code::EmptyBatch(step_type) => {
                tracing::error!("Building an empty batch for {step_type}")
            }
            Code::StateFatal(reason) => {
                tracing::error!("State error -> {reason}")
            }
        }
    }

    fn assert(&self) {
        let is_fatal = matches!(self, Code::Fatal(_, _) | Code::StateFatal(_));
        let is_failed = matches!(self, Code::ExecutionFailure(_, _));
        let is_skipped = matches!(self, Code::InvalidStep(_));
        let is_successful = matches!(self, Code::Success(_));

        let step_type = if let Some(step_type) = self.step_type() {
            step_type
        } else {
            return;
        };
        let details = json!({"outcome": self.code()});

        if is_fatal {
            match step_type {
                StepType::NewWalletKeyPair => assert_step!("Fatal NewWalletKeyPair", details),
                StepType::FaucetTransfer => assert_step!("Fatal FaucetTransfer", details),
                StepType::TransparentTransfer => assert_step!("Fatal TransparentTransfer", details),
                StepType::Bond => assert_step!("Fatal Bond", details),
                StepType::InitAccount => assert_step!("Fatal InitAccount", details),
                StepType::Redelegate => assert_step!("Fatal Redelegate", details),
                StepType::Unbond => assert_step!("Fatal Unbond", details),
                StepType::ClaimRewards => assert_step!("Fatal ClaimRewards", details),
                StepType::BatchBond => assert_step!("Fatal BatchBond", details),
                StepType::BatchRandom => assert_step!("Fatal BatchRandom", details),
                StepType::Shielding => assert_step!("Fatal Shielding", details),
                StepType::Shielded => assert_step!("Fatal Shielded", details),
                StepType::Unshielding => assert_step!("Fatal Unshielding", details),
                StepType::BecomeValidator => assert_step!("Fatal BecomeValidator", details),
                StepType::ChangeMetadata => assert_step!("Fatal ChangeMetadata", details),
                StepType::ChangeConsensusKeys => assert_step!("Fatal ChangeConsensusKeys", details),
                StepType::UpdateAccount => assert_step!("Fatal UpdateAccount", details),
                StepType::DeactivateValidator => assert_step!("Fatal DeactivateValidator", details),
                StepType::ReactivateValidator => assert_step!("Fatal ReactivateValidator", details),
                StepType::DefaultProposal => assert_step!("Fatal DefaultProposal", details),
                StepType::VoteProposal => assert_step!("Fatal VoteProposal", details),
            }
        } else if is_skipped {
            match step_type {
                StepType::NewWalletKeyPair => assert_step!("Skipped NewWalletKeyPair", details),
                StepType::FaucetTransfer => assert_step!("Skipped FaucetTransfer", details),
                StepType::TransparentTransfer => {
                    assert_step!("Skipped TransparentTransfer", details)
                }
                StepType::Bond => assert_step!("Skipped Bond", details),
                StepType::InitAccount => assert_step!("Skipped InitAccount", details),
                StepType::Redelegate => assert_step!("Skipped Redelegate", details),
                StepType::Unbond => assert_step!("Skipped Unbond", details),
                StepType::ClaimRewards => assert_step!("Skipped ClaimRewards", details),
                StepType::BatchBond => assert_step!("Skipped BatchBond", details),
                StepType::BatchRandom => assert_step!("Skipped BatchRandom", details),
                StepType::Shielding => assert_step!("Skipped Shielding", details),
                StepType::Shielded => assert_step!("Skipped Shielded", details),
                StepType::Unshielding => assert_step!("Skipped Unshielding", details),
                StepType::BecomeValidator => assert_step!("Skipped BecomeValidator", details),
                StepType::ChangeMetadata => assert_step!("Skipped ChangeMetadata", details),
                StepType::ChangeConsensusKeys => {
                    assert_step!("Skipped ChangeConsensusKeys", details)
                }
                StepType::UpdateAccount => assert_step!("Skipped UpdateAccount", details),
                StepType::DeactivateValidator => {
                    assert_step!("Skipped DeactivateValidator", details)
                }
                StepType::ReactivateValidator => {
                    assert_step!("Skipped ReactivateValidator", details)
                }
                StepType::DefaultProposal => assert_step!("Skipped DefaultProposal", details),
                StepType::VoteProposal => assert_step!("Skipped VoteProposal", details),
            }
        } else if is_successful {
            match step_type {
                StepType::NewWalletKeyPair => assert_step!("Done NewWalletKeyPair", details),
                StepType::FaucetTransfer => assert_step!("Done FaucetTransfer", details),
                StepType::TransparentTransfer => assert_step!("Done TransparentTransfer", details),
                StepType::Bond => assert_step!("Done Bond", details),
                StepType::InitAccount => assert_step!("Done InitAccount", details),
                StepType::Redelegate => assert_step!("Done Redelegate", details),
                StepType::Unbond => assert_step!("Done Unbond", details),
                StepType::ClaimRewards => assert_step!("Done ClaimRewards", details),
                StepType::BatchBond => assert_step!("Done BatchBond", details),
                StepType::BatchRandom => assert_step!("Done BatchRandom", details),
                StepType::Shielding => assert_step!("Done Shielding", details),
                StepType::Shielded => assert_step!("Done Shielded", details),
                StepType::Unshielding => assert_step!("Done Unshielding", details),
                StepType::BecomeValidator => assert_step!("Done BecomeValidator", details),
                StepType::ChangeMetadata => assert_step!("Done ChangeMetadata", details),
                StepType::ChangeConsensusKeys => assert_step!("Done ChangeConsensusKeys", details),
                StepType::UpdateAccount => assert_step!("Done UpdateAccount", details),
                StepType::DeactivateValidator => assert_step!("Done DeactivateValidator", details),
                StepType::ReactivateValidator => assert_step!("Done ReactivateValidator", details),
                StepType::DefaultProposal => assert_step!("Done DefaultProposal", details),
                StepType::VoteProposal => assert_step!("Done VoteProposal", details),
            }
        } else if is_failed {
            match step_type {
                StepType::NewWalletKeyPair => assert_step!("Failed NewWalletKeyPair", details),
                StepType::FaucetTransfer => assert_step!("Failed FaucetTransfer", details),
                StepType::TransparentTransfer => {
                    assert_step!("Failed TransparentTransfer", details)
                }
                StepType::Bond => assert_step!("Failed Bond", details),
                StepType::InitAccount => assert_step!("Failed InitAccount", details),
                StepType::Redelegate => assert_step!("Failed Redelegate", details),
                StepType::Unbond => assert_step!("Failed Unbond", details),
                StepType::ClaimRewards => assert_step!("Failed ClaimRewards", details),
                StepType::BatchBond => assert_step!("Failed BatchBond", details),
                StepType::BatchRandom => assert_step!("Failed BatchRandom", details),
                StepType::Shielding => assert_step!("Failed Shielding", details),
                StepType::Shielded => assert_step!("Failed Shielded", details),
                StepType::Unshielding => assert_step!("Failed Unshielding", details),
                StepType::BecomeValidator => assert_step!("Failed BecomeValidator", details),
                StepType::ChangeMetadata => assert_step!("Failed ChangeMetadata", details),
                StepType::ChangeConsensusKeys => {
                    assert_step!("Failed ChangeConsensusKeys", details)
                }
                StepType::UpdateAccount => assert_step!("Failed UpdateAccount", details),
                StepType::DeactivateValidator => {
                    assert_step!("Failed DeactivateValidator", details)
                }
                StepType::ReactivateValidator => {
                    assert_step!("Failed ReactivateValidator", details)
                }
                StepType::DefaultProposal => assert_step!("Failed DefaultProposal", details),
                StepType::VoteProposal => assert_step!("Failed VoteProposal", details),
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let exit_code = inner_main().await;

    exit_code.output_logs();

    exit_code.assert();

    std::process::exit(exit_code.code());
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
        &json!({"sha": env!("VERGEN_GIT_SHA")})
    );

    let (mut state, locked_file) = match State::load(config.id) {
        Ok(result) => result,
        Err(StateError::EmptyFile) => {
            tracing::warn!("State file is empty, creating new one");
            match State::create_new(config.id, config.seed) {
                Ok(result) => result,
                Err(e) => return Code::StateFatal(e),
            }
        }
        Err(e) => return Code::StateFatal(e),
    };

    tracing::info!("Using base dir: {}", state.base_dir.as_path().display());
    tracing::info!("Using seed: {}", state.seed);

    let url = Url::from_str(&config.rpc).expect("invalid RPC address");
    tracing::debug!("Opening connection to {url}");
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
                tracing::info!("No response from CometBFT, retrying... -> {err}");
            }
        }
        thread::sleep(Duration::from_secs(5));
    }

    let sdk = loop {
        match setup_sdk(&http_client, &state, &config).await {
            Ok(sdk) => break sdk,
            Err(_) => {
                tracing::info!("Setup SDK failed, retrying...")
            }
        }
        thread::sleep(Duration::from_secs(2));
    };

    let workload_executor = WorkloadExecutor::new();
    workload_executor.init(&sdk).await;

    let current_epoch = fetch_current_epoch(&sdk).await;

    let next_step = config.step_type;
    if !workload_executor.is_valid(&next_step, current_epoch, &state) {
        tracing::warn!("Invalid step: {next_step} -> {state:>?}");
        return Code::InvalidStep(next_step);
    }

    let init_block_height = fetch_current_block_height(&sdk).await;

    tracing::info!("Step is: {next_step}...");
    let tasks = match workload_executor.build(next_step, &sdk, &mut state).await {
        Ok(tasks) if tasks.is_empty() => {
            return Code::NoTask(next_step);
        }
        Ok(tasks) => tasks,
        Err(e) => {
            return Code::BuildFailure(next_step, e);
        }
    };
    tracing::info!("Built {next_step} -> {tasks:?}");

    let checks = workload_executor
        .build_check(&sdk, tasks.clone(), config.no_check)
        .await;
    tracing::info!("Built checks for {next_step}");

    let execution_height = match workload_executor.execute(&sdk, &tasks).await {
        Ok(result) => {
            let total_time_takes: u64 = result.iter().map(|execution| execution.time_taken).sum();
            tracing::info!("Execution took {total_time_takes}s...");
            result
                .iter()
                .filter_map(|execution| execution.execution_height)
                .max()
        }
        Err(e) if matches!(e, StepError::Execution(_)) => {
            state.update_failed_execution(&tasks); // remove fees
            return Code::ExecutionFailure(next_step, e);
        }
        Err(e) if matches!(e, StepError::Broadcast(_)) => {
            loop {
                let current_block_height = fetch_current_block_height(&sdk).await;
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

    let exit_code = match workload_executor
        .checks(&sdk, checks.clone(), execution_height)
        .await
    {
        Ok(_) => {
            tracing::info!("Checks were successful, updating state...");
            workload_executor.update_state(tasks, &mut state);
            Code::Success(next_step)
        }
        Err(e) => Code::Fatal(next_step, e),
    };

    tracing::info!("Statistics: {:>?}", state.stats);

    if let Err(e) = state.save(Some(locked_file)) {
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

async fn fetch_current_epoch(sdk: &Sdk) -> u64 {
    let client = sdk.namada.clone_client();
    loop {
        let latest_epoch = rpc::query_epoch(&client).await;
        if let Ok(epoch) = latest_epoch {
            return epoch.into();
        }
        sleep(Duration::from_secs_f64(1.0f64)).await
    }
}
