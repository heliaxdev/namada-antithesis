use std::{fmt::Display, str::FromStr, time::Instant};

use clap::ValueEnum;
use namada_sdk::{
    address::Address,
    args::TxBuilder,
    io::{Client, NamadaIo},
    key::common,
    rpc::{self},
    state::Epoch,
    token::{self},
    Namada,
};
use rand::{
    distributions::{Alphanumeric, DistString},
    Rng,
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

use crate::{
    build::{
        bond::build_bond, faucet_transfer::build_faucet_transfer,
        new_wallet_keypair::build_new_wallet_keypair,
        transparent_transfer::build_transparent_transfer,
    },
    check::Check,
    entities::Alias,
    execute::{
        bond::execute_bond, faucet_transfer::execute_faucet_transfer,
        new_wallet_keypair::execute_new_wallet_key_pair, reveal_pk::execute_reveal_pk,
        transparent_transfer::execute_transparent_transfer,
    },
    sdk::namada::Sdk,
    state::State,
    task::Task,
};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum StepType {
    NewWalletKeyPair,
    FaucetTransfer,
    TransparentTransfer,
    Bond,
}

impl Display for StepType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StepType::NewWalletKeyPair => write!(f, "wallet-key-pair"),
            StepType::FaucetTransfer => write!(f, "faucet-transfer"),
            StepType::TransparentTransfer => write!(f, "transparent-transfer"),
            StepType::Bond => write!(f, "bond"),
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
            StepType::Bond => state.any_account_with_min_balance(1),
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
            StepType::FaucetTransfer => build_faucet_transfer(state).await,
            StepType::TransparentTransfer => build_transparent_transfer(state).await,
            StepType::Bond => build_bond(sdk, state).await?,
        };
        Ok(steps)
    }

    pub async fn build_check(&self, sdk: &Sdk, tasks: Vec<Task>, state: &State) -> Vec<Check> {
        let config = Self::retry_config();

        let client = sdk.namada.client();
        let mut checks = vec![];
        for task in tasks {
            let check = match task {
                Task::NewWalletKeyPair(source) => vec![Check::RevealPk(source)],
                Task::FaucetTransfer(target, amount, _) => {
                    let wallet = sdk.namada.wallet.read().await;
                    let native_token_address = wallet.find_address("nam").unwrap().into_owned();
                    let target_address = wallet.find_address(&target.name).unwrap().into_owned();
                    drop(wallet);

                    let check = if let Ok(pre_balance) = tryhard::retry_fn(|| {
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
                        Check::BalanceTarget(target, pre_balance, amount, state.clone())
                    } else {
                        tracing::info!("retrying ...");
                        continue;
                    };

                    vec![check]
                }
                Task::TransparentTransfer(source, target, amount, _) => {
                    let wallet = sdk.namada.wallet.read().await;
                    let native_token_address = wallet.find_address("nam").unwrap().into_owned();
                    let source_address = wallet.find_address(&source.name).unwrap().into_owned();
                    let target_address = wallet.find_address(&target.name).unwrap().into_owned();
                    drop(wallet);

                    let source_check = if let Ok(pre_balance) = tryhard::retry_fn(|| {
                        rpc::get_token_balance(client, &native_token_address, &source_address, None)
                    })
                    .with_config(config)
                    .await
                    {
                        Check::BalanceSource(source, pre_balance, amount, state.clone())
                    } else {
                        tracing::info!("retrying ...");
                        continue;
                    };

                    let target_check = if let Ok(pre_balance) = tryhard::retry_fn(|| {
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
                        Check::BalanceTarget(target, pre_balance, amount, state.clone())
                    } else {
                        tracing::info!("retrying ...");
                        continue;
                    };

                    vec![source_check, target_check]
                }
                Task::Bond(source, validator, amount, epoch, _) => {
                    let wallet = sdk.namada.wallet.read().await;
                    let source_address = wallet.find_address(&source.name).unwrap().into_owned();

                    let validator_address = Address::from_str(&validator).unwrap();
                    let epoch = namada_sdk::state::Epoch::from(epoch);
                    drop(wallet);

                    let bond_check = if let Ok(pre_bond) = tryhard::retry_fn(|| {
                        rpc::get_bond_amount_at(
                            client,
                            &source_address,
                            &validator_address,
                            epoch.next().next(),
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
                        Check::Bond(source, validator, pre_bond, amount, state.clone())
                    } else {
                        tracing::info!("retrying ...");
                        continue;
                    };
                    vec![bond_check]
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
            sleep(Duration::from_secs_f64(0.5f64)).await
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
                                    "check_height": latest_block
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
                                return Err("BalanceTarget check error: post target amount is not equal to pre balance".to_string());
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
                                return Err(format!("BalanceTarget check error: post target amount not equal to pre balance: pre {}, post: {}, {}", pre_balance, post_amount, amount));
                            }
                        }
                        Err(e) => return Err(format!("BalanceTarget check error: {}", e)),
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
                    execute_transparent_transfer(sdk, source, target, amount, settings).await?
                }
                Task::Bond(source, validator, amount, _, settings) => {
                    execute_bond(sdk, source, validator, amount, settings).await?
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
        for task in tasks {
            match task {
                Task::NewWalletKeyPair(alias) => {
                    state.add_implicit_account(alias);
                }
                Task::FaucetTransfer(target, amount, settings) => {
                    let source_alias = Alias::faucet();
                    state.modify_balance(source_alias, target, amount);
                    state.modify_balance_fee(settings.gas_payer, settings.gas_limit);
                }
                Task::TransparentTransfer(source, target, amount, setting) => {
                    state.modify_balance(source, target, amount);
                    state.modify_balance_fee(setting.gas_payer, setting.gas_limit);
                }
                Task::Bond(source, validator, amount, _, setting) => {
                    state.modify_bond(source, validator, amount);
                    state.modify_balance_fee(setting.gas_payer, setting.gas_limit);
                }
            }
        }
    }

    async fn reveal_pk(sdk: &Sdk, public_key: common::PublicKey) -> Result<Option<u64>, StepError> {
        execute_reveal_pk(sdk, public_key).await
    }

    fn random_alias(state: &mut State) -> Alias {
        format!(
            "load-tester-{}",
            Alphanumeric.sample_string(&mut state.rng, 8)
        )
        .into()
    }

    fn random_between(from: u64, to: u64, state: &mut State) -> u64 {
        state.rng.gen_range(from..to)
    }

    fn retry_config() -> RetryFutureConfig<ExponentialBackoff, NoOnRetry> {
        RetryFutureConfig::new(4)
            .exponential_backoff(Duration::from_secs(1))
            .max_delay(Duration::from_secs(10))
    }
}
