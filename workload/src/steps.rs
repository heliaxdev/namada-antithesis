use std::{collections::HashMap, fmt::Display, str::FromStr, time::Instant};

use crate::{
    build::{
        batch::{build_bond_batch, build_random_batch},
        become_validator::build_become_validator,
        bond::build_bond,
        change_consensus_keys::build_change_consensus_keys,
        change_metadata::build_change_metadata,
        claim_rewards::build_claim_rewards,
        deactivate_validator::build_deactivate_validator,
        faucet_transfer::build_faucet_transfer,
        init_account::build_init_account,
        new_wallet_keypair::build_new_wallet_keypair,
        redelegate::build_redelegate,
        shielded_transfer::build_shielded_transfer,
        shielding::build_shielding,
        transparent_transfer::build_transparent_transfer,
        unbond::build_unbond,
        unshielding::build_unshielding,
        update_account::build_update_account,
    },
    build_checks,
    check::Check,
    entities::Alias,
    execute::{
        batch::execute_tx_batch,
        become_validator::build_tx_become_validator,
        bond::{build_tx_bond, execute_tx_bond},
        change_consensus_keys::{build_tx_change_consensus_key, execute_tx_change_consensus_key},
        change_metadata::build_tx_change_metadata,
        claim_rewards::{build_tx_claim_rewards, execute_tx_claim_rewards},
        deactivate_validator::{build_tx_deactivate_validator, execute_tx_deactivate_validator},
        faucet_transfer::execute_faucet_transfer,
        init_account::{build_tx_init_account, execute_tx_init_account},
        new_wallet_keypair::execute_new_wallet_key_pair,
        redelegate::{build_tx_redelegate, execute_tx_redelegate},
        reveal_pk::execute_reveal_pk,
        shielded::{build_tx_shielded_transfer, execute_tx_shielded_transfer},
        shielding::{build_tx_shielding, execute_tx_shielding},
        transparent_transfer::{build_tx_transparent_transfer, execute_tx_transparent_transfer},
        unbond::{build_tx_unbond, execute_tx_unbond},
        unshielding::{build_tx_unshielding, execute_tx_unshielding},
        update_account::{build_tx_update_account, execute_tx_update_account},
    },
    sdk::namada::Sdk,
    state::State,
    task::Task,
};
use clap::ValueEnum;
use namada_sdk::{
    address::Address,
    io::Client,
    key::common,
    proof_of_stake::types::ValidatorState,
    rpc::{self},
    state::Epoch,
    token::{self},
};
use serde_json::json;
use thiserror::Error;
use tokio::time::{sleep, Duration};
use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};

#[derive(Error, Debug, Clone)]
pub enum StepError {
    #[error("building an empty batch")]
    EmptyBatch,
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
    #[error("shield-sync `{0}`")]
    ShieldSync(String),
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum StepType {
    NewWalletKeyPair,
    FaucetTransfer,
    TransparentTransfer,
    Bond,
    InitAccount,
    Redelegate,
    Unbond,
    ClaimRewards,
    BatchBond,
    BatchRandom,
    Shielding,
    Shielded,
    Unshielding,
    BecomeValidator,
    ChangeMetadata,
    ChangeConsensusKeys,
    UpdateAccount,
    DeactivateValidator,
}

impl Display for StepType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StepType::NewWalletKeyPair => write!(f, "wallet-key-pair"),
            StepType::FaucetTransfer => write!(f, "faucet-transfer"),
            StepType::TransparentTransfer => write!(f, "transparent-transfer"),
            StepType::Bond => write!(f, "bond"),
            StepType::InitAccount => write!(f, "init-account"),
            StepType::Redelegate => write!(f, "redelegate"),
            StepType::Unbond => write!(f, "unbond"),
            StepType::ClaimRewards => write!(f, "claim-rewards"),
            StepType::Shielding => write!(f, "shielding"),
            StepType::BatchRandom => write!(f, "batch-random"),
            StepType::BatchBond => write!(f, "batch-bond"),
            StepType::Shielded => write!(f, "shielded"),
            StepType::Unshielding => write!(f, "unshielding"),
            StepType::BecomeValidator => write!(f, "become-validator"),
            StepType::ChangeMetadata => write!(f, "change-metadata"),
            StepType::ChangeConsensusKeys => write!(f, "change-consensus-keys"),
            StepType::UpdateAccount => write!(f, "update-account"),
            StepType::DeactivateValidator => write!(f, "deactivate-validator"),
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
        let client = sdk.namada.clone_client();
        let wallet = sdk.namada.wallet.write().await;
        let faucet_address = wallet.find_address("faucet").unwrap().into_owned();
        let nam_address = wallet.find_address("nam").unwrap().into_owned();
        let faucet_public_key = wallet.find_public_key("faucet").unwrap().to_owned();
        drop(wallet);

        loop {
            if let Ok(res) =
                rpc::get_token_balance(&client, &nam_address, &faucet_address, None).await
            {
                if res.is_zero() {
                    tracing::error!("Faucet has no money RIP.");
                    std::process::exit(1);
                } else {
                    tracing::info!("Faucet has $$$ ({})", res);
                    break;
                }
            } else {
                tracing::warn!("Retry querying for  faucet balance...");
                sleep(Duration::from_secs(2)).await;
            }
        }

        loop {
            if let Ok(res) = rpc::is_public_key_revealed(&client, &faucet_address).await {
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
            StepType::Unbond => state.any_bond(),
            StepType::InitAccount => state.min_n_implicit_accounts(3),
            StepType::Redelegate => state.any_bond(),
            StepType::ClaimRewards => state.any_bond(),
            StepType::Shielding => state.any_account_with_min_balance(2),
            StepType::BatchBond => state.min_n_account_with_min_balance(3, 2),
            StepType::BatchRandom => {
                state.min_n_account_with_min_balance(3, 2) && state.min_bonds(3)
            }
            StepType::Shielded => {
                state.at_least_masp_accounts(2)
                    && state.at_least_masp_account_with_minimal_balance(1, 2)
            }
            StepType::Unshielding => {
                state.at_least_masp_account_with_minimal_balance(1, 2)
                    && state.min_n_implicit_accounts(1)
            }
            StepType::BecomeValidator => state.min_n_enstablished_accounts(1),
            StepType::ChangeMetadata => state.min_n_validators(1),
            StepType::ChangeConsensusKeys => state.min_n_validators(1),
            StepType::DeactivateValidator => state.min_n_validators(1),
            StepType::UpdateAccount => {
                state.min_n_enstablished_accounts(1) && state.min_n_implicit_accounts(3)
            }
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
            StepType::InitAccount => build_init_account(state).await?,
            StepType::Redelegate => build_redelegate(sdk, state).await?,
            StepType::Unbond => build_unbond(sdk, state).await?,
            StepType::ClaimRewards => build_claim_rewards(state),
            StepType::Shielding => build_shielding(state).await?,
            StepType::BatchBond => build_bond_batch(sdk, 3, state).await?,
            StepType::BatchRandom => build_random_batch(sdk, 3, state).await?,
            StepType::Shielded => build_shielded_transfer(state).await?,
            StepType::Unshielding => build_unshielding(state).await?,
            StepType::BecomeValidator => build_become_validator(state).await?,
            StepType::ChangeMetadata => build_change_metadata(state).await?,
            StepType::ChangeConsensusKeys => build_change_consensus_keys(state).await?,
            StepType::DeactivateValidator => build_deactivate_validator(state).await?,
            StepType::UpdateAccount => build_update_account(state).await?,
        };
        Ok(steps)
    }

    pub async fn build_check(
        &self,
        sdk: &Sdk,
        tasks: Vec<Task>,
        state: &State,
        no_check: bool,
    ) -> Vec<Check> {
        if no_check {
            return vec![];
        }
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
                Task::InitAccount(alias, sources, threshold, _) => {
                    build_checks::init_account::init_account_build_checks(
                        sdk,
                        alias,
                        sources,
                        threshold,
                        retry_config,
                        state,
                    )
                    .await
                }
                Task::Redelegate(source, from, to, amount, epoch, _) => {
                    build_checks::redelegate::redelegate(
                        sdk,
                        source,
                        from,
                        to,
                        amount,
                        epoch,
                        retry_config,
                        state,
                    )
                    .await
                }
                Task::Unbond(source, validator, amount, epoch, _) => {
                    build_checks::unbond::unbond(
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
                Task::ClaimRewards(_source, _validator, _) => {
                    vec![]
                }
                Task::ShieldedTransfer(source, target, amount, _) => {
                    build_checks::shielded_transfer::shielded_transfer(
                        sdk,
                        source,
                        target,
                        amount,
                        false,
                        retry_config,
                        state,
                    )
                    .await
                }
                Task::Shielding(source, target, amount, _) => {
                    build_checks::shielding::shielding(
                        sdk,
                        source,
                        target,
                        amount,
                        false,
                        retry_config,
                        state,
                    )
                    .await
                }
                Task::Unshielding(source, target, amount, _) => {
                    build_checks::unshielding::unshielding(
                        sdk,
                        source,
                        target,
                        amount,
                        false,
                        retry_config,
                        state,
                    )
                    .await
                }
                Task::BecomeValidator(source, _, _, _, _, _, _, _) => {
                    build_checks::become_validator::become_validator(source).await
                }
                Task::ChangeMetadata(_, _, _, _, _, _, _) => {
                    vec![]
                }
                Task::ChangeConsensusKeys(_, _, _) => {
                    vec![]
                }
                Task::UpdateAccount(target, sources, threshold, _) => {
                    build_checks::update_account::update_account_build_checks(
                        sdk,
                        target,
                        sources,
                        threshold,
                        retry_config,
                        state,
                    )
                    .await
                }
                Task::DeactivateValidator(target, _) => {
                    build_checks::deactivate_validator::deactivate_validator_build_checks(
                        sdk,
                        target,
                        retry_config,
                        state,
                    )
                    .await
                }
                Task::Batch(tasks, _) => {
                    let mut checks = vec![];

                    let mut reveal_pks: HashMap<Alias, Alias> = HashMap::default();
                    let mut balances: HashMap<Alias, i64> = HashMap::default();
                    let mut shielded_balances: HashMap<Alias, i64> = HashMap::default();
                    let mut bonds: HashMap<String, (u64, i64)> = HashMap::default();

                    for task in tasks {
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
                                bonds
                                    .entry(format!("{}@{}", source.name, validator))
                                    .and_modify(|(_epoch, bond_amount)| {
                                        *bond_amount += *amount as i64
                                    })
                                    .or_insert((*epoch, *amount as i64));
                                balances
                                    .entry(source.clone())
                                    .and_modify(|balance| *balance -= *amount as i64)
                                    .or_insert(-(*amount as i64));
                            }
                            Task::Unbond(source, validator, amount, epoch, _task_settings) => {
                                bonds
                                    .entry(format!("{}@{}", source.name, validator))
                                    .and_modify(|(_epoch, bond_amount)| {
                                        *bond_amount -= *amount as i64
                                    })
                                    .or_insert((*epoch, -(*amount as i64)));
                            }
                            Task::Redelegate(source, from, to, amount, epoch, _task_settings) => {
                                bonds
                                    .entry(format!("{}@{}", source.name, to))
                                    .and_modify(|(_epoch, bond_amount)| {
                                        *bond_amount += *amount as i64
                                    })
                                    .or_insert((*epoch, *amount as i64));
                                bonds
                                    .entry(format!("{}@{}", source.name, from))
                                    .and_modify(|(_epoch, bond_amount)| {
                                        *bond_amount -= *amount as i64
                                    })
                                    .or_insert((*epoch, -(*amount as i64)));
                            }
                            Task::ShieldedTransfer(source, target, amount, _task_settings) => {
                                shielded_balances
                                    .entry(source.clone())
                                    .and_modify(|balance| *balance -= *amount as i64)
                                    .or_insert(-(*amount as i64));
                                shielded_balances
                                    .entry(target.clone())
                                    .and_modify(|balance| *balance += *amount as i64)
                                    .or_insert(*amount as i64);
                            }
                            Task::Shielding(source, target, amount, _task_settings) => {
                                balances
                                    .entry(source.clone())
                                    .and_modify(|balance| *balance -= *amount as i64)
                                    .or_insert(-(*amount as i64));
                                shielded_balances
                                    .entry(target.clone())
                                    .and_modify(|balance| *balance += *amount as i64)
                                    .or_insert(*amount as i64);
                            }
                            Task::Unshielding(source, target, amount, _task_settings) => {
                                balances
                                    .entry(source.clone())
                                    .and_modify(|balance| *balance += *amount as i64)
                                    .or_insert(-(*amount as i64));
                                shielded_balances
                                    .entry(target.clone())
                                    .and_modify(|balance| *balance -= *amount as i64)
                                    .or_insert(*amount as i64);
                            }
                            Task::ClaimRewards(_source, _validator, _task_settings) => {}
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

                    for (key, (epoch, amount)) in bonds {
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
                            if amount > 0 {
                                checks.push(Check::BondIncrease(
                                    Alias::from(source),
                                    validator.to_owned(),
                                    pre_bond,
                                    amount.unsigned_abs(),
                                    state.clone(),
                                ));
                            } else {
                                checks.push(Check::BondDecrease(
                                    Alias::from(source),
                                    validator.to_owned(),
                                    pre_bond,
                                    amount.unsigned_abs(),
                                    state.clone(),
                                ));
                            }
                        }
                    }

                    for (alias, amount) in shielded_balances {
                        if let Ok(Some(pre_balance)) = build_checks::utils::get_shielded_balance(
                            sdk,
                            alias.clone(),
                            None,
                            true,
                        )
                        .await
                        {
                            if amount >= 0 {
                                checks.push(Check::BalanceShieldedTarget(
                                    alias,
                                    pre_balance,
                                    amount.unsigned_abs(),
                                    state.clone(),
                                ));
                            } else {
                                checks.push(Check::BalanceShieldedSource(
                                    alias,
                                    pre_balance,
                                    amount.unsigned_abs(),
                                    state.clone(),
                                ));
                            }
                        }
                    }

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
        let client = sdk.namada.clone_client();

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
            sleep(Duration::from_secs_f64(2.0f64)).await
        };

        for check in checks {
            tracing::info!("Running {} check...", check.to_string());
            match check {
                Check::RevealPk(alias) => {
                    let wallet = sdk.namada.wallet.read().await;
                    let source = wallet.find_address(&alias.name).unwrap().into_owned();
                    drop(wallet);

                    match tryhard::retry_fn(|| rpc::is_public_key_revealed(&client, &source))
                        .with_config(config)
                        .await
                    {
                        Ok(was_pk_revealed) => {
                            antithesis_sdk::assert_always!(
                                was_pk_revealed,
                                "The public key was revealed correctly.",
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
                        rpc::get_token_balance(
                            &client,
                            &native_token_address,
                            &target_address,
                            None,
                        )
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
                                    "BalanceTarget check error: balance is overflowing".to_string()
                                );
                            };
                            antithesis_sdk::assert_always!(
                                post_amount.eq(&check_balance),
                                "Balance target increased.",
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
                Check::BalanceShieldedSource(target, pre_balance, amount, pre_state) => {
                    match build_checks::utils::get_shielded_balance(
                        sdk,
                        target.clone(),
                        Some(execution_height),
                        false,
                    )
                    .await
                    {
                        Ok(Some(post_balance)) => {
                            let check_balance = if let Some(balance) =
                                pre_balance.checked_sub(token::Amount::from_u64(amount))
                            {
                                balance
                            } else {
                                return Err(
                                    "BalanceShieldedSource check error: balance is underflowing"
                                        .to_string(),
                                );
                            };
                            antithesis_sdk::assert_always!(
                                post_balance.eq(&check_balance),
                                "BalanceShielded source decreased.",
                                &json!({
                                    "source_alias": target,
                                    "pre_balance": pre_balance,
                                    "amount": amount,
                                    "post_balance": post_balance,
                                    "pre_state": pre_state,
                                    "timeout": random_timeout,
                                    "execution_height": execution_height,
                                    "check_height": latest_block
                                })
                            );
                            if !post_balance.eq(&check_balance) {
                                tracing::error!(
                                    "{}",
                                    json!({
                                        "source_alias": target,
                                        "pre_balance": pre_balance,
                                        "amount": amount,
                                        "post_balance": post_balance,
                                        "pre_state": pre_state,
                                        "timeout": random_timeout,
                                        "execution_height": execution_height,
                                        "check_height": latest_block
                                    })
                                );
                                return Err(format!("BalanceShieldedSource check error: post source amount is not equal to pre balance - amount: {} - {} = {} != {}", pre_balance, amount, check_balance, post_balance));
                            }
                        }
                        Ok(None) => {
                            antithesis_sdk::assert_unreachable!(
                                "BalanceShieldedSource target doesn't exist.",
                                &json!({
                                    "source_alias": target,
                                    "pre_balance": pre_balance,
                                    "amount": amount,
                                    "pre_state": pre_state,
                                    "timeout": random_timeout,
                                    "execution_height": execution_height,
                                    "check_height": latest_block
                                })
                            );
                            return Err("BalanceShieldedSource check error: amount doesn't exist"
                                .to_string());
                        }
                        Err(e) => {
                            return Err(format!("BalanceShieldedSource check error: {e}",));
                        }
                    };
                }
                Check::BalanceShieldedTarget(target, pre_balance, amount, pre_state) => {
                    match build_checks::utils::get_shielded_balance(
                        sdk,
                        target.clone(),
                        Some(execution_height),
                        true,
                    )
                    .await
                    {
                        Ok(Some(post_balance)) => {
                            let check_balance = if let Some(balance) =
                                pre_balance.checked_add(token::Amount::from_u64(amount))
                            {
                                balance
                            } else {
                                return Err(
                                    "BalanceShieldedTarget check error: balance is overflowing"
                                        .to_string(),
                                );
                            };
                            antithesis_sdk::assert_always!(
                                post_balance.eq(&check_balance),
                                "BalanceShielded target increased.",
                                &json!({
                                    "target_alias": target,
                                    "pre_balance": pre_balance,
                                    "amount": amount,
                                    "post_balance": post_balance,
                                    "pre_state": pre_state,
                                    "timeout": random_timeout,
                                    "execution_height": execution_height,
                                    "check_height": latest_block
                                })
                            );
                            if !post_balance.eq(&check_balance) {
                                tracing::error!(
                                    "{}",
                                    json!({
                                        "target_alias": target,
                                        "pre_balance": pre_balance,
                                        "amount": amount,
                                        "post_balance": post_balance,
                                        "pre_state": pre_state,
                                        "timeout": random_timeout,
                                        "execution_height": execution_height,
                                        "check_height": latest_block
                                    })
                                );
                                return Err(format!("BalanceShieldedTarget check error: post target amount is not equal to pre balance: pre {}, post: {}, {}", pre_balance, post_balance, amount));
                            }
                        }
                        Ok(None) => {
                            antithesis_sdk::assert_unreachable!(
                                "BalanceShieldedTarget target doesn't exist.",
                                &json!({
                                    "target_alias": target,
                                    "pre_balance": pre_balance,
                                    "amount": amount,
                                    "pre_state": pre_state,
                                    "timeout": random_timeout,
                                    "execution_height": execution_height,
                                    "check_height": latest_block
                                })
                            );
                            return Err("BalanceShieldedTarget check error: amount doesn't exist"
                                .to_string());
                        }
                        Err(e) => {
                            return Err(format!("BalanceShieldedTarget check error: {e}",));
                        }
                    };
                }
                Check::BalanceSource(target, pre_balance, amount, pre_state) => {
                    let wallet = sdk.namada.wallet.read().await;
                    let native_token_address = wallet.find_address("nam").unwrap().into_owned();
                    let target_address = wallet.find_address(&target.name).unwrap().into_owned();
                    drop(wallet);

                    match tryhard::retry_fn(|| {
                        rpc::get_token_balance(
                            &client,
                            &native_token_address,
                            &target_address,
                            None,
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
                                "Balance source decreased.",
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
                Check::BondIncrease(target, validator, pre_bond, amount, pre_state) => {
                    let wallet = sdk.namada.wallet.read().await;
                    let source_address = wallet.find_address(&target.name).unwrap().into_owned();

                    let validator_address = Address::from_str(&validator).unwrap();

                    let epoch = if let Ok(epoch) = tryhard::retry_fn(|| rpc::query_epoch(&client))
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
                            &client,
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
                                return Err(
                                    "Bond increase check error: bond is negative".to_string()
                                );
                            };
                            antithesis_sdk::assert_always!(
                                post_bond.eq(&check_bond),
                                "Bond source increased.",
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
                                return Err(format!("Bond increase check error: post target amount is not equal to pre balance: pre {}, post {}, amount: {}", pre_bond, post_bond, amount));
                            }
                        }
                        Err(e) => return Err(format!("Bond check error: {}", e)),
                    }
                }
                Check::BondDecrease(target, validator, pre_bond, amount, pre_state) => {
                    let wallet = sdk.namada.wallet.read().await;
                    let source_address = wallet.find_address(&target.name).unwrap().into_owned();

                    let validator_address = Address::from_str(&validator).unwrap();

                    let epoch = if let Ok(epoch) = tryhard::retry_fn(|| rpc::query_epoch(&client))
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
                            &client,
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
                                pre_bond.checked_sub(token::Amount::from_u64(amount))
                            {
                                bond
                            } else {
                                return Err(
                                    "Bond decrease check error: bond is negative".to_string()
                                );
                            };
                            antithesis_sdk::assert_always!(
                                post_bond.eq(&check_bond),
                                "Bond source decreased.",
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
                                return Err(format!("Bond decrease check error: post target amount is not equal to pre balance: pre {}, post {}, amount: {}", pre_bond, post_bond, amount));
                            }
                        }
                        Err(e) => return Err(format!("Bond check error: {}", e)),
                    }
                }
                Check::AccountExist(target, threshold, sources, pre_state) => {
                    let wallet = sdk.namada.wallet.read().await;
                    let source_address = wallet.find_address(&target.name).unwrap().into_owned();
                    wallet.save().unwrap();
                    drop(wallet);

                    match tryhard::retry_fn(|| rpc::get_account_info(&client, &source_address))
                        .with_config(config)
                        .on_retry(|attempt, _, error| {
                            let error = error.to_string();
                            async move {
                                tracing::info!("Retry {} due to {}...", attempt, error);
                            }
                        })
                        .await
                    {
                        Ok(Some(account)) => {
                            let is_threshold_ok = account.threshold == threshold as u8;
                            let is_sources_ok =
                                sources.len() == account.public_keys_map.idx_to_pk.len();
                            antithesis_sdk::assert_always!(
                                is_sources_ok && is_threshold_ok,
                                "OnChain account is valid.",
                                &json!({
                                    "target_alias": target,
                                    "target": source_address.to_pretty_string(),
                                    "account": account,
                                    "threshold": threshold,
                                    "sources": sources,
                                    "pre_state": pre_state,
                                    "timeout": random_timeout,
                                    "execution_height": execution_height,
                                    "check_height": latest_block
                                })
                            );
                            if !is_sources_ok || !is_threshold_ok {
                                tracing::error!(
                                    "{}",
                                    json!({
                                        "target_alias": target,
                                        "target": source_address.to_pretty_string(),
                                        "account": account,
                                        "threshold": threshold,
                                        "sources": sources,
                                        "pre_state": pre_state,
                                        "timeout": random_timeout,
                                        "execution_height": execution_height,
                                        "check_height": latest_block
                                    })
                                );
                                return Err(format!(
                                    "AccountExist check error: account {} is invalid",
                                    source_address
                                ));
                            }
                        }
                        Ok(None) => {
                            antithesis_sdk::assert_unreachable!(
                                "OnChain account doesn't exist.",
                                &json!({
                                    "target_alias": target,
                                    "target": source_address.to_pretty_string(),
                                    "account": "",
                                    "threshold": threshold,
                                    "sources": sources,
                                    "pre_state": pre_state,
                                    "timeout": random_timeout,
                                    "execution_height": execution_height,
                                    "check_height": latest_block
                                })
                            );
                            return Err(format!(
                                "AccountExist check error: account {} doesn't exist",
                                target.name
                            ));
                        }
                        Err(e) => return Err(format!("AccountExist check error: {}", e)),
                    };
                }
                Check::IsValidatorAccount(target) => {
                    let wallet = sdk.namada.wallet.read().await;
                    let source_address = wallet.find_address(&target.name).unwrap().into_owned();
                    wallet.save().unwrap();
                    drop(wallet);

                    let is_validator = rpc::is_validator(&client, &source_address)
                        .await
                        .unwrap_or_default();
                    antithesis_sdk::assert_always!(
                        is_validator,
                        "OnChain account is a validator.",
                        &json!({
                            "target_alias": target,
                            "target": source_address.to_pretty_string(),
                            "timeout": random_timeout,
                            "execution_height": execution_height,
                            "check_height": latest_block
                        })
                    );
                }
                Check::ValidatorStatus(target, status) => {
                    let wallet = sdk.namada.wallet.read().await;
                    let source_address = wallet.find_address(&target.name).unwrap().into_owned();
                    wallet.save().unwrap();
                    drop(wallet);

                    let epoch = if let Ok(epoch) = tryhard::retry_fn(|| rpc::query_epoch(&client))
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
                        rpc::get_validator_state(
                            &client,
                            &source_address,
                            Some(epoch.next().next()),
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
                        Ok((Some(state), _epoch)) => {
                            let is_valid_status = match status {
                                crate::check::ValidatorStatus::Active => {
                                    state.ne(&ValidatorState::Inactive)
                                }
                                crate::check::ValidatorStatus::Inactive => {
                                    state.eq(&ValidatorState::Inactive)
                                }
                            };
                            antithesis_sdk::assert_always!(
                                is_valid_status,
                                "Validator status correctly changed.",
                                &json!({
                                    "target_alias": target,
                                    "target": source_address.to_pretty_string(),
                                    "to_status": status.to_string(),
                                    "timeout": random_timeout,
                                    "execution_height": execution_height,
                                    "check_height": latest_block
                                })
                            );
                        }
                        Ok((None, _epoch)) => {
                            antithesis_sdk::assert_unreachable!(
                                "OnChain validator account doesn't exist.",
                                &json!({
                                    "target_alias": target,
                                    "target": source_address.to_pretty_string(),
                                    "timeout": random_timeout,
                                    "execution_height": execution_height,
                                    "check_height": latest_block
                                })
                            );
                            return Err(format!(
                                "Validator status check error: validator {} doesn't exist",
                                target.name
                            ));
                        }
                        Err(e) => return Err(format!("ValidatorStatus check error: {}", e)),
                    };
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
                Task::Bond(source, validator, amount, _epoch, settings) => {
                    let (mut tx, signing_data, tx_args) =
                        build_tx_bond(sdk, source, validator, amount, settings).await?;
                    execute_tx_bond(sdk, &mut tx, signing_data, &tx_args).await?
                }
                Task::InitAccount(source, sources, threshold, settings) => {
                    let (mut tx, signing_data, tx_args) =
                        build_tx_init_account(sdk, source, sources, threshold, settings).await?;
                    execute_tx_init_account(sdk, &mut tx, signing_data, &tx_args).await?
                }
                Task::Redelegate(source, from, to, amount, _epoch, settings) => {
                    let (mut tx, signing_data, tx_args) =
                        build_tx_redelegate(sdk, source, from, to, amount, settings).await?;
                    execute_tx_redelegate(sdk, &mut tx, signing_data, &tx_args).await?
                }
                Task::Unbond(source, validator, amount, _epoch, settings) => {
                    let (mut tx, signing_data, tx_args) =
                        build_tx_unbond(sdk, source, validator, amount, settings).await?;
                    execute_tx_unbond(sdk, &mut tx, signing_data, &tx_args).await?
                }
                Task::ClaimRewards(source, validator, settings) => {
                    let (mut tx, signing_data, tx_args) =
                        build_tx_claim_rewards(sdk, source, validator, settings).await?;
                    execute_tx_claim_rewards(sdk, &mut tx, signing_data, &tx_args).await?
                }
                Task::ShieldedTransfer(source, target, amount, settings) => {
                    let (mut tx, signing_data, tx_args) =
                        build_tx_shielded_transfer(sdk, source, target, amount, settings).await?;
                    execute_tx_shielded_transfer(sdk, &mut tx, signing_data, &tx_args).await?
                }
                Task::Shielding(source, target, amount, settings) => {
                    let (mut tx, signing_data, tx_args) =
                        build_tx_shielding(sdk, source, target, amount, settings).await?;
                    execute_tx_shielding(sdk, &mut tx, signing_data, &tx_args).await?
                }
                Task::Unshielding(source, target, amount, settings) => {
                    let (mut tx, signing_data, tx_args) =
                        build_tx_unshielding(sdk, source, target, amount, settings).await?;
                    execute_tx_unshielding(sdk, &mut tx, signing_data, &tx_args).await?
                }
                Task::DeactivateValidator(target, settings) => {
                    let (mut tx, signing_data, tx_args) =
                        build_tx_deactivate_validator(sdk, target, settings).await?;
                    execute_tx_deactivate_validator(sdk, &mut tx, signing_data, &tx_args).await?
                }
                Task::ChangeMetadata(
                    source,
                    website,
                    email,
                    discord,
                    description,
                    avatar,
                    settings,
                ) => {
                    let (mut tx, signing_data, tx_args) = build_tx_change_metadata(
                        sdk,
                        source,
                        website,
                        email,
                        discord,
                        description,
                        avatar,
                        settings,
                    )
                    .await?;
                    execute_tx_shielding(sdk, &mut tx, signing_data, &tx_args).await?
                }
                Task::ChangeConsensusKeys(source, alias, settings) => {
                    let (mut tx, signing_data, tx_args) =
                        build_tx_change_consensus_key(sdk, source, alias, settings).await?;
                    execute_tx_change_consensus_key(sdk, &mut tx, signing_data, &tx_args).await?
                }
                Task::UpdateAccount(target, sources, threshold, settings) => {
                    let (mut tx, signing_data, tx_args) =
                        build_tx_update_account(sdk, target, sources, threshold, settings).await?;
                    execute_tx_update_account(sdk, &mut tx, signing_data, &tx_args).await?
                }
                Task::BecomeValidator(alias, t, t1, t2, t3, comm, max_comm_change, settings) => {
                    let (mut tx, signing_data, tx_args) = build_tx_become_validator(
                        sdk,
                        alias,
                        t,
                        t1,
                        t2,
                        t3,
                        comm,
                        max_comm_change,
                        settings,
                    )
                    .await?;
                    execute_tx_shielding(sdk, &mut tx, signing_data, &tx_args).await?
                }
                Task::Batch(tasks, task_settings) => {
                    let mut txs = vec![];
                    for task in tasks {
                        let (tx, signing_data, _) = match task {
                            Task::TransparentTransfer(source, target, amount, settings) => {
                                build_tx_transparent_transfer(sdk, source, target, amount, settings)
                                    .await?
                            }
                            Task::Bond(source, validator, amount, _epoch, settings) => {
                                build_tx_bond(sdk, source, validator, amount, settings).await?
                            }
                            Task::Redelegate(source, from, to, amount, _epoch, task_settings) => {
                                build_tx_redelegate(sdk, source, from, to, amount, task_settings)
                                    .await?
                            }
                            Task::Unbond(source, validator, amount, _epoch, settings) => {
                                build_tx_unbond(sdk, source, validator, amount, settings).await?
                            }
                            Task::ShieldedTransfer(source, target, amount, settings) => {
                                build_tx_shielded_transfer(sdk, source, target, amount, settings)
                                    .await?
                            }
                            Task::Shielding(source, target, amount, settings) => {
                                build_tx_shielding(sdk, source, target, amount, settings).await?
                            }
                            Task::ClaimRewards(source, validator, settings) => {
                                build_tx_claim_rewards(sdk, source, validator, settings).await?
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
