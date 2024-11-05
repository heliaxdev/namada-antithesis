use std::{collections::HashMap, fmt::Display, str::FromStr, time::Instant};

use crate::{
    build::{
        batch::{build_bond_batch, build_random_batch},
        bond::build_bond,
        faucet_transfer::build_faucet_transfer,
        new_wallet_keypair::build_new_wallet_keypair,
        transparent_transfer::build_transparent_transfer,
    },
    build_checks,
    check::Check,
    entities::Alias,
    execute::{
        batch::execute_tx_batch,
        bond::{build_tx_bond, execute_tx_bond},
        faucet_transfer::execute_faucet_transfer,
        new_wallet_keypair::execute_new_wallet_key_pair,
        reveal_pk::execute_reveal_pk,
        transparent_transfer::{build_tx_transparent_transfer, execute_tx_transparent_transfer},
    },
    sdk::namada::Sdk,
    state::State,
    task::Task,
};
use clap::ValueEnum;
use namada_sdk::{
    address::Address,
    io::{Client, NamadaIo},
    key::common,
    rpc::{self},
    state::Epoch,
    token::{self},
};
use serde_json::json;
use thiserror::Error;
use tokio::time::{sleep, Duration};
use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};

#[derive(Error, Debug)]
pub enum StepError {
    #[error("error wallet `{0}`")]
    Wallet(String),
    #[error("error building tx `{0}`")]
    Build(String),
    #[error("error fetching shielded context data `{0}`")]
    ShieldedSync(String),
    #[error("error broadcasting tx `{0}`")]
    Broadcast(String),
    #[error("error executing tx `{0}`")]
    Execution(String),
    #[error("error calling rpc `{0}`")]
    Rpc(String),
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum StepType {
    NewWalletKeyPair,
    FaucetTransfer,
    TransparentTransfer,
    Bond,
    BatchBond,
    BatchRandom,
}

impl Display for StepType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StepType::NewWalletKeyPair => write!(f, "wallet-key-pair"),
            StepType::FaucetTransfer => write!(f, "faucet-transfer"),
            StepType::TransparentTransfer => write!(f, "transparent-transfer"),
            StepType::Bond => write!(f, "bond"),
            StepType::BatchRandom => write!(f, "batch-random"),
            StepType::BatchBond => write!(f, "batch-bond"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ExecutionResult {
    pub time_taken: u64,
    pub execution_height: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct WorkloadExecutor {}

impl Default for WorkloadExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkloadExecutor {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn init(&self, sdk: &Sdk) {
        let client = sdk.namada.client();
        let wallet = sdk.namada.wallet.write().await;
        let faucet_address = wallet.find_address("faucet").unwrap().into_owned();
        let faucet_public_key = wallet.find_public_key("faucet").unwrap().to_owned();

        loop {
            if let Ok(res) = rpc::is_public_key_revealed(client, &faucet_address).await {
                if !res {
                    let _ = Self::reveal_pk(sdk, faucet_public_key.clone()).await;
                } else {
                    break;
                }
            } else {
                tracing::warn!("Retry revealing faucet pk...");
                sleep(Duration::from_secs(2)).await;
            }
        }
    }

    pub fn is_valid(&self, step_type: &StepType, state: &State) -> bool {
        match step_type {
            StepType::NewWalletKeyPair => true,
            StepType::FaucetTransfer => state.any_account(),
            StepType::TransparentTransfer => {
                state.at_least_accounts(2) && state.any_account_can_make_transfer()
            }
            StepType::Bond => state.any_account_with_min_balance(2),
            StepType::BatchBond => state.min_n_account_with_min_balance(3, 2),
            StepType::BatchRandom => state.min_n_account_with_min_balance(3, 2),
        }
    }

    pub async fn build(
        &self,
        step_type: StepType,
        sdk: &Sdk,
        state: &mut State,
    ) -> Result<Vec<Task>, StepError> {
        let steps = match step_type {
            StepType::NewWalletKeyPair => build_new_wallet_keypair(state).await,
            StepType::FaucetTransfer => build_faucet_transfer(state).await?,
            StepType::TransparentTransfer => build_transparent_transfer(state).await?,
            StepType::Bond => build_bond(sdk, state).await?,
            StepType::BatchBond => build_bond_batch(sdk, 3, state).await?,
            StepType::BatchRandom => build_random_batch(sdk, 3, state).await?,
        };
        Ok(steps)
    }

    pub async fn build_check(&self, sdk: &Sdk, tasks: Vec<Task>, state: &State) -> Vec<Check> {
        let retry_config = Self::retry_config();

        let mut checks = vec![];
        for task in tasks {
            let check = match task {
                Task::NewWalletKeyPair(source) => vec![Check::RevealPk(source)],
                Task::FaucetTransfer(target, amount, _) => {
                    build_checks::faucet::faucet_build_check(
                        sdk,
                        target,
                        amount,
                        retry_config,
                        state,
                    )
                    .await
                }
                Task::TransparentTransfer(source, target, amount, _) => {
                    build_checks::transparent_transfer::transparent_transfer(
                        sdk,
                        source,
                        target,
                        amount,
                        retry_config,
                        state,
                    )
                    .await
                }
                Task::Bond(source, validator, amount, epoch, _) => {
                    build_checks::bond::bond(
                        sdk,
                        source,
                        validator,
                        amount,
                        epoch,
                        retry_config,
                        state,
                    )
                    .await
                }
                Task::Batch(tasks, _) => {
                    let mut checks = vec![];

                    let mut reveal_pks: HashMap<Alias, Alias> = HashMap::default();
                    let mut balances: HashMap<Alias, i64> = HashMap::default();
                    let mut bond: HashMap<String, (u64, u64)> = HashMap::default();

                    for task in tasks {
                        println!("{:>?}", task);
                        match &task {
                            Task::NewWalletKeyPair(source) => {
                                reveal_pks.insert(source.clone(), source.to_owned());
                            }
                            Task::FaucetTransfer(target, amount, _task_settings) => {
                                balances
                                    .entry(target.clone())
                                    .and_modify(|balance| *balance += *amount as i64)
                                    .or_insert(*amount as i64);
                            }
                            Task::TransparentTransfer(source, target, amount, _task_settings) => {
                                balances
                                    .entry(target.clone())
                                    .and_modify(|balance| *balance += *amount as i64)
                                    .or_insert(*amount as i64);
                                balances
                                    .entry(source.clone())
                                    .and_modify(|balance| *balance -= *amount as i64)
                                    .or_insert(-(*amount as i64));
                            }
                            Task::Bond(source, validator, amount, epoch, _task_settings) => {
                                bond.entry(format!("{}@{}", source.name, validator))
                                    .and_modify(|(_epoch, balance)| *balance += amount)
                                    .or_insert((*epoch, *amount));
                                balances
                                    .entry(source.clone())
                                    .and_modify(|balance| *balance -= *amount as i64)
                                    .or_insert(-(*amount as i64));
                            }
                            _ => panic!(),
                        };
                    }

                    for (_, source) in reveal_pks {
                        checks.push(Check::RevealPk(source));
                    }

                    for (alias, amount) in balances {
                        if let Some(pre_balance) =
                            build_checks::utils::get_balance(sdk, alias.clone(), retry_config).await
                        {
                            if amount >= 0 {
                                checks.push(Check::BalanceTarget(
                                    alias,
                                    pre_balance,
                                    amount.unsigned_abs(),
                                    state.clone(),
                                ));
                            } else {
                                checks.push(Check::BalanceSource(
                                    alias,
                                    pre_balance,
                                    amount.unsigned_abs(),
                                    state.clone(),
                                ));
                            }
                        }
                    }

                    for (key, (epoch, amount)) in bond {
                        let (source, validator) = key.split_once('@').unwrap();
                        if let Some(pre_bond) = build_checks::utils::get_bond(
                            sdk,
                            Alias::from(source),
                            validator.to_owned(),
                            epoch,
                            retry_config,
                        )
                        .await
                        {
                            checks.push(Check::Bond(
                                Alias::from(source),
                                validator.to_owned(),
                                pre_bond,
                                amount,
                                state.clone(),
                            ));
                        }
                    }
                    println!("{:>?}", checks);
                    checks
                }
            };
            checks.extend(check)
        }
        checks
    }

    pub async fn checks(
        &self,
        sdk: &Sdk,
        checks: Vec<Check>,
        execution_height: Option<u64>,
    ) -> Result<(), String> {
        let config = Self::retry_config();
        let random_timeout = 0.0f64;
        let client = sdk.namada.client();

        if checks.is_empty() {
            return Ok(());
        }

        let execution_height = if let Some(height) = execution_height {
            height
        } else {
            return Ok(());
        };

        let latest_block = loop {
            let latest_block = client.latest_block().await;
            if let Ok(block) = latest_block {
                let current_height = block.block.last_commit.unwrap().height.value();
                let block_height = current_height;
                if block_height >= execution_height {
                    break current_height;
                } else {
                    tracing::info!(
                        "Waiting for block height: {}, currently at: {}",
                        execution_height,
                        block_height
                    );
                }
            }
            sleep(Duration::from_secs_f64(1.0f64)).await
        };

        for check in checks {
            match check {
                Check::RevealPk(alias) => {
                    let wallet = sdk.namada.wallet.read().await;
                    let source = wallet.find_address(&alias.name).unwrap().into_owned();
                    drop(wallet);

                    match tryhard::retry_fn(|| rpc::is_public_key_revealed(client, &source))
                        .with_config(config)
                        .await
                    {
                        Ok(was_pk_revealed) => {
                            antithesis_sdk::assert_always!(
                                was_pk_revealed,
                                "The public key was not released correctly.",
                                &json!({
                                    "public-key": source.to_pretty_string(),
                                    "timeout": random_timeout,
                                    "execution_height": execution_height,
                                    "check_height": latest_block,
                                })
                            );
                            if !was_pk_revealed {
                                return Err(format!(
                                    "RevealPk check error: pk for {} was not revealed",
                                    source.to_pretty_string()
                                ));
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                "{}",
                                json!({
                                    "public-key": source.to_pretty_string(),
                                    "timeout": random_timeout,
                                    "execution_height": execution_height,
                                    "check_height": latest_block,
                                })
                            );
                            return Err(format!("RevealPk check error: {}", e));
                        }
                    }
                }
                Check::BalanceTarget(target, pre_balance, amount, pre_state) => {
                    let wallet = sdk.namada.wallet.read().await;
                    let native_token_address = wallet.find_address("nam").unwrap().into_owned();
                    let target_address = wallet.find_address(&target.name).unwrap().into_owned();
                    drop(wallet);

                    match tryhard::retry_fn(|| {
                        rpc::get_token_balance(client, &native_token_address, &target_address, None)
                    })
                    .with_config(config)
                    .on_retry(|attempt, _, error| {
                        let error = error.to_string();
                        async move {
                            tracing::warn!("Retry {} due to {}...", attempt, error);
                        }
                    })
                    .await
                    {
                        Ok(post_amount) => {
                            let check_balance = if let Some(balance) =
                                pre_balance.checked_add(token::Amount::from_u64(amount))
                            {
                                balance
                            } else {
                                return Err(
                                    "BalanceTarget check error: balance is negative".to_string()
                                );
                            };
                            antithesis_sdk::assert_always!(
                                post_amount.eq(&check_balance),
                                "Balance target didn't increase.",
                                &json!({
                                    "target_alias": target,
                                    "target": target_address.to_pretty_string(),
                                    "pre_balance": pre_balance,
                                    "amount": amount,
                                    "post_balance": post_amount,
                                    "pre_state": pre_state,
                                    "timeout": random_timeout,
                                    "execution_height": execution_height,
                                    "check_height": latest_block
                                })
                            );
                            if !post_amount.eq(&check_balance) {
                                tracing::error!(
                                    "{}",
                                    json!({
                                        "target_alias": target,
                                        "target": target_address.to_pretty_string(),
                                        "pre_balance": pre_balance,
                                        "amount": amount,
                                        "post_balance": post_amount,
                                        "pre_state": pre_state,
                                        "timeout": random_timeout,
                                        "execution_height": execution_height,
                                        "check_height": latest_block
                                    })
                                );
                                return Err(format!("BalanceTarget check error: post target amount is not equal to pre balance: pre {}, post: {}, {}", pre_balance, post_amount, amount));
                            }
                        }
                        Err(e) => return Err(format!("BalanceTarget check error: {}", e)),
                    }
                }
                Check::BalanceSource(target, pre_balance, amount, pre_state) => {
                    let wallet = sdk.namada.wallet.read().await;
                    let native_token_address = wallet.find_address("nam").unwrap().into_owned();
                    let target_address = wallet.find_address(&target.name).unwrap().into_owned();
                    drop(wallet);

                    match tryhard::retry_fn(|| {
                        rpc::get_token_balance(client, &native_token_address, &target_address, None)
                    })
                    .with_config(config)
                    .on_retry(|attempt, _, error| {
                        let error = error.to_string();
                        async move {
                            tracing::info!("Retry {} due to {}...", attempt, error);
                        }
                    })
                    .await
                    {
                        Ok(post_amount) => {
                            let check_balance = if let Some(balance) =
                                pre_balance.checked_sub(token::Amount::from_u64(amount))
                            {
                                balance
                            } else {
                                return Err(
                                    "BalanceTarget check error: balance is negative".to_string()
                                );
                            };
                            antithesis_sdk::assert_always!(
                                post_amount.eq(&check_balance),
                                "Balance source didn't decrease.",
                                &json!({
                                    "target_alias": target,
                                    "target": target_address.to_pretty_string(),
                                    "pre_balance": pre_balance,
                                    "amount": amount,
                                    "post_balance": post_amount,
                                    "pre_state": pre_state,
                                    "timeout": random_timeout,
                                    "execution_height": execution_height,
                                    "check_height": latest_block
                                })
                            );
                            if !post_amount.eq(&check_balance) {
                                tracing::error!(
                                    "{}",
                                    json!({
                                        "target_alias": target,
                                        "target": target_address.to_pretty_string(),
                                        "pre_balance": pre_balance,
                                        "amount": amount,
                                        "post_balance": post_amount,
                                        "pre_state": pre_state,
                                        "timeout": random_timeout,
                                        "execution_height": execution_height,
                                        "check_height": latest_block
                                    })
                                );
                                return Err(format!("BalanceSource check error: post target amount not equal to pre balance: pre {}, post: {}, {}", pre_balance, post_amount, amount));
                            }
                        }
                        Err(e) => return Err(format!("BalanceSource check error: {}", e)),
                    }
                }
                Check::Bond(target, validator, pre_bond, amount, pre_state) => {
                    let wallet = sdk.namada.wallet.read().await;
                    let source_address = wallet.find_address(&target.name).unwrap().into_owned();

                    let validator_address = Address::from_str(&validator).unwrap();

                    let epoch = if let Ok(epoch) = tryhard::retry_fn(|| rpc::query_epoch(client))
                        .with_config(config)
                        .on_retry(|attempt, _, error| {
                            let error = error.to_string();
                            async move {
                                tracing::info!("Retry {} due to {}...", attempt, error);
                            }
                        })
                        .await
                    {
                        epoch
                    } else {
                        continue;
                    };

                    match tryhard::retry_fn(|| {
                        rpc::get_bond_amount_at(
                            client,
                            &source_address,
                            &validator_address,
                            Epoch(epoch.0 + 2),
                        )
                    })
                    .with_config(config)
                    .on_retry(|attempt, _, error| {
                        let error = error.to_string();
                        async move {
                            tracing::info!("Retry {} due to {}...", attempt, error);
                        }
                    })
                    .await
                    {
                        Ok(post_bond) => {
                            let check_bond = if let Some(bond) =
                                pre_bond.checked_add(token::Amount::from_u64(amount))
                            {
                                bond
                            } else {
                                return Err("Bond check error: bond is negative".to_string());
                            };
                            antithesis_sdk::assert_always!(
                                post_bond.eq(&check_bond),
                                "Bond source didn't increase.",
                                &json!({
                                    "target_alias": target,
                                    "target": source_address.to_pretty_string(),
                                    "validator": validator_address.to_pretty_string(),
                                    "pre_bond": pre_bond,
                                    "amount": amount,
                                    "post_bond": post_bond,
                                    "pre_state": pre_state,
                                    "epoch": epoch,
                                    "timeout": random_timeout,
                                    "execution_height": execution_height,
                                    "check_height": latest_block
                                })
                            );
                            if !post_bond.eq(&check_bond) {
                                tracing::error!(
                                    "{}",
                                    json!({
                                        "target_alias": target,
                                        "target": source_address.to_pretty_string(),
                                        "validator": validator_address.to_pretty_string(),
                                        "pre_bond": pre_bond,
                                        "amount": amount,
                                        "post_bond": post_bond,
                                        "pre_state": pre_state,
                                        "epoch": epoch,
                                        "timeout": random_timeout,
                                        "execution_height": execution_height,
                                        "check_height": latest_block
                                    })
                                );
                                return Err(format!("Bond check error: post target amount is not equal to pre balance: pre {}, post {}, amount: {}", pre_bond, post_bond, amount));
                            }
                        }
                        Err(e) => return Err(format!("Bond check error: {}", e)),
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn execute(
        &self,
        sdk: &Sdk,
        tasks: Vec<Task>,
    ) -> Result<Vec<ExecutionResult>, StepError> {
        let mut execution_results = vec![];

        for task in tasks {
            let now = Instant::now();
            let execution_height = match task {
                Task::NewWalletKeyPair(alias) => {
                    let public_key = execute_new_wallet_key_pair(sdk, alias).await?;
                    Self::reveal_pk(sdk, public_key).await?
                }
                Task::FaucetTransfer(target, amount, settings) => {
                    execute_faucet_transfer(sdk, target, amount, settings).await?
                }
                Task::TransparentTransfer(source, target, amount, settings) => {
                    let (mut tx, signing_data, tx_args) =
                        build_tx_transparent_transfer(sdk, source, target, amount, settings)
                            .await?;
                    execute_tx_transparent_transfer(sdk, &mut tx, signing_data, &tx_args).await?
                }
                Task::Bond(source, validator, amount, _, settings) => {
                    let (mut tx, signing_data, tx_args) =
                        build_tx_bond(sdk, source, validator, amount, settings).await?;
                    execute_tx_bond(sdk, &mut tx, signing_data, &tx_args).await?
                }
                Task::Batch(tasks, task_settings) => {
                    let mut txs = vec![];
                    for task in tasks {
                        let (tx, signing_data, _) = match task {
                            Task::TransparentTransfer(source, target, amount, settings) => {
                                build_tx_transparent_transfer(sdk, source, target, amount, settings)
                                    .await?
                            }
                            Task::Bond(source, validator, amount, _, settings) => {
                                build_tx_bond(sdk, source, validator, amount, settings).await?
                            }
                            _ => panic!(),
                        };
                        txs.push((tx, signing_data));
                    }

                    execute_tx_batch(sdk, txs, task_settings).await?
                }
            };
            let execution_result = ExecutionResult {
                time_taken: now.elapsed().as_secs(),
                execution_height,
            };
            execution_results.push(execution_result);
        }

        Ok(execution_results)
    }

    pub fn update_state(&self, tasks: Vec<Task>, state: &mut State) {
        state.update(tasks, true);
    }

    async fn reveal_pk(sdk: &Sdk, public_key: common::PublicKey) -> Result<Option<u64>, StepError> {
        execute_reveal_pk(sdk, public_key).await
    }

    fn retry_config() -> RetryFutureConfig<ExponentialBackoff, NoOnRetry> {
        RetryFutureConfig::new(4)
            .exponential_backoff(Duration::from_secs(1))
            .max_delay(Duration::from_secs(10))
    }
}
