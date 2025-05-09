use std::{env, time::Duration};

use antithesis_sdk::antithesis_init;
use clap::Parser;
use namada_chain_workload::code::Code;
use namada_chain_workload::config::{AppConfig, Args};
use namada_chain_workload::context::Ctx;
use namada_chain_workload::error::CheckError;
use namada_chain_workload::executor::WorkloadExecutor;
use namada_chain_workload::state::{State, StateError};
use namada_chain_workload::utils::base_dir;
use serde_json::json;
use tokio::time::sleep;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    let args = Args::parse();
    if args.setup_complete {
        antithesis_sdk::lifecycle::setup_complete(&json!({
            "commit_sha": env!("VERGEN_GIT_SHA")
        }));
        std::process::exit(0);
    }

    let code = inner_main(args).await;

    code.output_logs();

    code.assert();

    std::process::exit(code.code());
}

async fn inner_main(args: Args) -> Code {
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

    let config = match AppConfig::load(args.config) {
        Ok(config) => config,
        Err(e) => return Code::ConfigFatal(e.to_string()),
    };

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

    let next_step = args.step_type;
    tracing::info!("Using config: {config:#?}");
    tracing::info!("StepType: {next_step}");

    // just to report the workload version
    antithesis_sdk::assert_always!(
        true,
        "ID should be greater than 0",
        &json!({
            "base dir": base_dir(),
            "commit_sha": env!("VERGEN_GIT_SHA")
        })
    );

    let ctx = loop {
        match Ctx::new(&config).await {
            Ok(ctx) => break ctx,
            Err(_) => {
                tracing::info!("Setup Context failed, retrying...");
                sleep(Duration::from_secs(2)).await;
            }
        }
    };

    let mut workload_executor = WorkloadExecutor::new(ctx, state);
    if let Err(e) = workload_executor.init().await {
        return Code::InitFatal(e);
    }

    match workload_executor.is_valid(&next_step).await {
        Ok(true) => {}
        _ => {
            tracing::warn!(
                "Invalid step: {next_step} -> {:>?}",
                workload_executor.state()
            );
            return Code::Skip(next_step);
        }
    }

    tracing::info!("Step is: {next_step}...");
    let tasks = match workload_executor.build_tasks(&next_step).await {
        Ok(tasks) if tasks.is_empty() => {
            return Code::NoTask(next_step);
        }
        Ok(tasks) => tasks,
        Err(e) => {
            return Code::StepFailure(next_step, e);
        }
    };
    tracing::info!("Built tasks for {next_step}");

    let checks = if args.no_check {
        vec![]
    } else {
        match workload_executor.build_check(&tasks).await {
            Ok(checks) => checks,
            Err(e) => return Code::TaskFailure(next_step, e),
        }
    };
    tracing::info!("Built checks for {next_step}");

    let (result, fees) = workload_executor.execute(&tasks).await;
    workload_executor.apply_fee_payments(&fees);

    let execution_height = match result {
        Ok(height) => height,
        Err(e) => {
            // Update the state file for the fee payment of the failure transactions
            if let Err(e) = workload_executor.state().save(Some(locked_file)) {
                return Code::StateFatal(e);
            }

            return Code::TaskFailure(next_step, e);
        }
    };

    tracing::info!("Execution were successful, updating state...");
    if let Err(e) = workload_executor
        .post_execute(&tasks, execution_height)
        .await
    {
        return Code::TaskFailure(next_step, e);
    }

    let exit_code = match workload_executor
        .checks(checks, execution_height, &fees)
        .await
    {
        Ok(_) => Code::Success(next_step),
        Err(e) if matches!(e, CheckError::State(_)) => Code::Fatal(next_step, e),
        Err(e) => Code::CheckFailure(next_step, e),
    };

    tracing::info!("Statistics: {:>?}", workload_executor.state().stats);

    if let Err(e) = workload_executor.state().save(Some(locked_file)) {
        return Code::StateFatal(e);
    }

    exit_code
}
