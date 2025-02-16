use std::{collections::HashMap, str::FromStr, time::Instant};

use crate::{
    build::{
        batch::{build_bond_batch, build_random_batch},
        become_validator::build_become_validator,
        bond::build_bond,
        change_consensus_keys::build_change_consensus_keys,
        change_metadata::build_change_metadata,
        claim_rewards::build_claim_rewards,
        deactivate_validator::build_deactivate_validator,
        default_proposal::build_default_proposal,
        faucet_transfer::build_faucet_transfer,
        init_account::build_init_account,
        new_wallet_keypair::build_new_wallet_keypair,
        reactivate_validator::build_reactivate_validator,
        redelegate::build_redelegate,
        shielded_transfer::build_shielded_transfer,
        shielding::build_shielding,
        transparent_transfer::build_transparent_transfer,
        unbond::build_unbond,
        unshielding::build_unshielding,
        update_account::build_update_account,
        vote::build_vote,
    },
    build_checks,
    check::Check,
    constants::{MIN_TRANSFER_BALANCE, PROPOSAL_DEPOSIT},
    entities::Alias,
    execute::reveal_pk::execute_reveal_pk,
    sdk::namada::Sdk,
    step::StepType,
    state::State,
    task::Task,
};
use namada_sdk::{
    address::Address,
    proof_of_stake::types::ValidatorState,
    rpc::{self},
    state::Epoch,
    token::{self},
};
use serde_json::json;
use thiserror::Error;
use tokio::time::{sleep, Duration};
use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};

#[derive(Error, Debug, Clone, PartialEq)]
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
    #[error("state check: `{0}`")]
    StateCheck(String),
}

#[derive(Clone, Debug)]
pub struct ExecutionResult {
    pub time_taken: u64,
    pub execution_height: Option<u64>,
}

pub struct WorkloadExecutor {
    sdk: Sdk,
    state: State,
}

impl WorkloadExecutor {
    pub fn new(sdk: Sdk, state: State) -> Self {
        Self { sdk, state }
    }

    pub fn sdk(&self) -> &Sdk {
        &self.sdk
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub async fn fetch_current_block_height(&self) -> u64 {
        loop {
            if let Ok(Some(latest_block)) = rpc::query_block(&self.sdk.namada.client).await {
                return latest_block.height.into()
            }
            sleep(Duration::from_secs(1)).await
        }
    }

    async fn fetch_current_epoch(&self) -> u64 {
        loop {
            let latest_epoch = rpc::query_epoch(&self.sdk.namada.client).await;
            if let Ok(epoch) = latest_epoch {
                return epoch.into();
            }
            sleep(Duration::from_secs(1)).await
        }
    }

    pub async fn init(&self) {
        let client = &self.sdk.namada.client;
        let wallet = self.sdk.namada.wallet.read().await;
        let faucet_address = wallet.find_address("faucet").unwrap().into_owned();
        let nam_address = wallet.find_address("nam").unwrap().into_owned();
        let faucet_public_key = wallet.find_public_key("faucet").unwrap().to_owned();
        drop(wallet);

        loop {
            if let Ok(res) =
                rpc::get_token_balance(client, &nam_address, &faucet_address, None).await
            {
                if res.is_zero() {
                    tracing::error!("Faucet has no money RIP.");
                    std::process::exit(1);
                } else {
                    tracing::info!("Faucet has $$$ ({})", res);
                    break;
                }
            }
            tracing::warn!("Retry querying for faucet balance...");
            sleep(Duration::from_secs(2)).await;
        }

        loop {
            if let Ok(is_revealed) = rpc::is_public_key_revealed(client, &faucet_address).await {
                if is_revealed {
                    break;
                }
            }
            if let Ok(Some(_)) = execute_reveal_pk(&self.sdk, faucet_public_key.clone()).await {
                break;
            }
            tracing::warn!("Retry revealing faucet pk...");
            sleep(Duration::from_secs(2)).await;
        }
    }

    pub async fn is_valid(&self, step_type: &StepType) -> bool {
        match step_type {
            StepType::NewWalletKeyPair => true,
            StepType::FaucetTransfer => self.state.any_account(),
            StepType::TransparentTransfer => {
                self.state.at_least_accounts(2) && self.state.any_account_can_make_transfer()
            }
            StepType::Bond => self.state.any_account_with_min_balance(MIN_TRANSFER_BALANCE),
            StepType::Unbond => self.state.any_bond(),
            StepType::InitAccount => self.state.min_n_implicit_accounts(3),
            StepType::Redelegate => self.state.any_bond(),
            StepType::ClaimRewards => self.state.any_bond(),
            StepType::Shielding => self.state.any_account_with_min_balance(MIN_TRANSFER_BALANCE),
            StepType::BatchBond => self.state.min_n_account_with_min_balance(3, MIN_TRANSFER_BALANCE),
            StepType::BatchRandom => {
                self.state.min_n_account_with_min_balance(3, MIN_TRANSFER_BALANCE) && self.state.min_bonds(3)
            }
            StepType::Shielded => {
                self.state.at_least_masp_accounts(2)
                    && self.state.at_least_masp_account_with_minimal_balance(1, MIN_TRANSFER_BALANCE)
            }
            StepType::Unshielding => {
                self.state.at_least_masp_account_with_minimal_balance(1, MIN_TRANSFER_BALANCE)
                    && self.state.min_n_implicit_accounts(1)
            }
            StepType::BecomeValidator => self.state.min_n_enstablished_accounts(1),
            StepType::ChangeMetadata => self.state.min_n_validators(1),
            StepType::ChangeConsensusKeys => self.state.min_n_validators(1),
            StepType::DeactivateValidator => self.state.min_n_validators(1),
            StepType::UpdateAccount => {
                self.state.min_n_enstablished_accounts(1) && self.state.min_n_implicit_accounts(3)
            }
            StepType::ReactivateValidator => self.state.min_n_deactivated_validators(1),
            StepType::DefaultProposal => self.state.any_account_with_min_balance(PROPOSAL_DEPOSIT),
            StepType::VoteProposal => {
                let current_epoch = self.fetch_current_epoch().await;
                self.state.any_bond() && self.state.any_votable_proposal(current_epoch)
            }
        }
    }

    pub async fn build(
        &self,
        step_type: StepType,
    ) -> Result<Vec<Task>, StepError> {
        let steps = match step_type {
            StepType::NewWalletKeyPair => build_new_wallet_keypair(&mut self.state).await,
            StepType::FaucetTransfer => build_faucet_transfer(&mut self.state).await?,
            StepType::TransparentTransfer => build_transparent_transfer(&mut self.state).await?,
            StepType::Bond => build_bond(&self.sdk, &mut self.state).await?,
            StepType::InitAccount => build_init_account(&mut self.state).await?,
            StepType::Redelegate => build_redelegate(&self.sdk, &mut self.state).await?,
            StepType::Unbond => build_unbond(&self.sdk, &mut self.state).await?,
            StepType::ClaimRewards => build_claim_rewards(&mut self.state),
            StepType::Shielding => build_shielding(&mut self.state).await?,
            StepType::BatchBond => build_bond_batch(&self.sdk, 3, &mut self.state).await?,
            StepType::BatchRandom => build_random_batch(&self.sdk, 3, &mut self.state).await?,
            StepType::Shielded => build_shielded_transfer(&mut self.state).await?,
            StepType::Unshielding => build_unshielding(&mut self.state).await?,
            StepType::BecomeValidator => build_become_validator(&mut self.state).await?,
            StepType::ChangeMetadata => build_change_metadata(&mut self.state).await?,
            StepType::ChangeConsensusKeys => build_change_consensus_keys(&mut self.state).await?,
            StepType::DeactivateValidator => build_deactivate_validator(&mut self.state).await?,
            StepType::UpdateAccount => build_update_account(&mut self.state).await?,
            StepType::ReactivateValidator => build_reactivate_validator(&mut self.state).await?,
            StepType::DefaultProposal => build_default_proposal(&self.sdk, &mut self.state).await?,
            StepType::VoteProposal => build_vote(&self.sdk, &mut self.state).await?,
        };
        Ok(steps)
    }

    pub async fn build_check(&self, tasks: Vec<Task>, no_check: bool) -> Vec<Check> {
        let mut checks = vec![];
        for task in tasks {
            let check = match task {
                Task::NewWalletKeyPair(source) => vec![Check::RevealPk(source)],
                Task::FaucetTransfer(target, amount, _) => {
                    build_checks::faucet::faucet_build_check(&self.sdk, target, amount, retry_config)
                        .await
                }
                Task::TransparentTransfer(source, target, amount, _) => {
                    build_checks::transparent_transfer::transparent_transfer(
                        &self.sdk,
                        source,
                        target,
                        amount,
                        retry_config,
                    )
                    .await
                }
                Task::Bond(source, validator, amount, epoch, _) => {
                    build_checks::bond::bond(&self.sdk, source, validator, amount, epoch, retry_config)
                        .await
                }
                Task::InitAccount(alias, sources, threshold, _) => {
                    build_checks::init_account::init_account_build_checks(
                        &self.sdk,
                        alias,
                        sources,
                        threshold,
                        retry_config,
                    )
                    .await
                }
                Task::Redelegate(source, from, to, amount, epoch, _) => {
                    build_checks::redelegate::redelegate(
                        &self.sdk,
                        source,
                        from,
                        to,
                        amount,
                        epoch,
                        retry_config,
                    )
                    .await
                }
                Task::Unbond(source, validator, amount, epoch, _) => {
                    build_checks::unbond::unbond(
                        &self.sdk,
                        source,
                        validator,
                        amount,
                        epoch,
                        retry_config,
                    )
                    .await
                }
                Task::ClaimRewards(_source, _validator, _) => {
                    vec![]
                }
                Task::ShieldedTransfer(source, target, amount, _) => {
                    build_checks::shielded_transfer::shielded_transfer(
                        &self.sdk,
                        source,
                        target,
                        amount,
                        false,
                        retry_config,
                    )
                    .await
                }
                Task::Shielding(source, target, amount, _) => {
                    build_checks::shielding::shielding(
                        &self.sdk,
                        source,
                        target,
                        amount,
                        false,
                        retry_config,
                    )
                    .await
                }
                Task::Unshielding(source, target, amount, _) => {
                    build_checks::unshielding::unshielding(
                        &self.sdk,
                        source,
                        target,
                        amount,
                        false,
                        retry_config,
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
                        &self.sdk,
                        target,
                        sources,
                        threshold,
                        retry_config,
                    )
                    .await
                }
                Task::DefaultProposal(source, _start_epoch, _end_epoch, _grace_epoch, _) => {
                    build_checks::proposal::proposal(sdk, source, retry_config).await
                }
                Task::Vote(_, _, _, _) => {
                    vec![]
                }
                Task::DeactivateValidator(target, _) => {
                    build_checks::deactivate_validator::deactivate_validator_build_checks(
                        &self.sdk,
                        target,
                        retry_config,
                    )
                    .await
                }
                Task::ReactivateValidator(target, _) => {
                    build_checks::reactivate_validator::reactivate_validator_build_checks(
                        &self.sdk,
                        target,
                        retry_config,
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
                            build_checks::utils::get_balance(&self.sdk, alias.clone(), retry_config).await
                        {
                            if amount >= 0 {
                                checks.push(Check::BalanceTarget(
                                    alias,
                                    pre_balance,
                                    amount.unsigned_abs(),
                                ));
                            } else {
                                checks.push(Check::BalanceSource(
                                    alias,
                                    pre_balance,
                                    amount.unsigned_abs(),
                                ));
                            }
                        }
                    }

                    for (key, (epoch, amount)) in bonds {
                        let (source, validator) = key.split_once('@').unwrap();
                        if let Some(pre_bond) = build_checks::utils::get_bond(
                            &self.sdk,
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
                                ));
                            } else {
                                checks.push(Check::BondDecrease(
                                    Alias::from(source),
                                    validator.to_owned(),
                                    pre_bond,
                                    amount.unsigned_abs(),
                                ));
                            }
                        }
                    }

                    for (alias, amount) in shielded_balances {
                        if let Ok(Some(pre_balance)) = build_checks::utils::get_shielded_balance(
                            &self.sdk,
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
                                ));
                            } else {
                                checks.push(Check::BalanceShieldedSource(
                                    alias,
                                    pre_balance,
                                    amount.unsigned_abs(),
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
    ) -> Result<(), StepError> {
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

        let height = loop {
            let current_height = self.fetch_current_block_height().await;
            if current_height >= execution_height {
                break current_height;
            } else {
                tracing::info!(
                    "Waiting for block height: {}, currently at: {}",
                    execution_height,
                    current_height
                );
            }
            sleep(Duration::from_secs(2)).await
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
                            let public_key = source.to_pretty_string();
                            antithesis_sdk::assert_always!(
                                was_pk_revealed,
                                "The public key was revealed correctly.",
                                &json!({
                                    "public_key": public_key,
                                    "timeout": random_timeout,
                                    "execution_height": execution_height,
                                    "check_height": height,
                                })
                            );
                            if !was_pk_revealed {
                                return Err(StepError::StateCheck(format!(
                                    "RevealPk check error: pk for {public_key} was not revealed",
                                )));
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                "{}",
                                json!({
                                    "public_key": source.to_pretty_string(),
                                    "timeout": random_timeout,
                                    "execution_height": execution_height,
                                    "check_height": latest_block,
                                })
                            );
                            return Err(StepError::StateCheck(format!(
                                "RevealPk check error: {e}"
                            )));
                        }
                    }
                }
                Check::BalanceTarget(target, pre_balance, amount) => {
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
                                return Err(StepError::StateCheck(
                                    "BalanceTarget check error: balance is overflowing".to_string(),
                                ));
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
                                        "timeout": random_timeout,
                                        "execution_height": execution_height,
                                        "check_height": latest_block
                                    })
                                );
                                return Err(StepError::StateCheck(format!("BalanceTarget check error: post target amount is not equal to pre balance: pre {pre_balance}, post: {post_amount}, {amount}")));
                            }
                        }
                        Err(e) => {
                            return Err(StepError::StateCheck(format!(
                                "BalanceTarget check error: {e}"
                            )))
                        }
                    }
                }
                Check::BalanceShieldedSource(target, pre_balance, amount) => {
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
                                return Err(StepError::StateCheck(
                                    "BalanceShieldedSource check error: balance is underflowing"
                                        .to_string(),
                                ));
                            };
                            antithesis_sdk::assert_always!(
                                post_balance.eq(&check_balance),
                                "BalanceShielded source decreased.",
                                &json!({
                                    "source_alias": target,
                                    "pre_balance": pre_balance,
                                    "amount": amount,
                                    "post_balance": post_balance,
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
                                        "timeout": random_timeout,
                                        "execution_height": execution_height,
                                        "check_height": latest_block
                                    })
                                );
                                return Err(StepError::StateCheck(format!("BalanceShieldedSource check error: post source amount is not equal to pre balance - amount: {pre_balance} - {amount} = {check_balance} != {post_balance}")));
                            }
                        }
                        Ok(None) => {
                            antithesis_sdk::assert_unreachable!(
                                "BalanceShieldedSource target doesn't exist.",
                                &json!({
                                    "source_alias": target,
                                    "pre_balance": pre_balance,
                                    "amount": amount,
                                    "timeout": random_timeout,
                                    "execution_height": execution_height,
                                    "check_height": latest_block
                                })
                            );
                            return Err(StepError::StateCheck(
                                "BalanceShieldedSource check error: amount doesn't exist"
                                    .to_string(),
                            ));
                        }
                        Err(e) => {
                            return Err(StepError::StateCheck(format!(
                                "BalanceShieldedSource check error: {e}"
                            )));
                        }
                    };
                }
                Check::BalanceShieldedTarget(target, pre_balance, amount) => {
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
                                return Err(StepError::StateCheck(
                                    "BalanceShieldedTarget check error: balance is overflowing"
                                        .to_string(),
                                ));
                            };
                            antithesis_sdk::assert_always!(
                                post_balance.eq(&check_balance),
                                "BalanceShielded target increased.",
                                &json!({
                                    "target_alias": target,
                                    "pre_balance": pre_balance,
                                    "amount": amount,
                                    "post_balance": post_balance,
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
                                        "timeout": random_timeout,
                                        "execution_height": execution_height,
                                        "check_height": latest_block
                                    })
                                );
                                return Err(StepError::StateCheck(format!("BalanceShieldedTarget check error: post target amount is not equal to pre balance: pre {pre_balance}, post: {post_balance}, {amount}")));
                            }
                        }
                        Ok(None) => {
                            antithesis_sdk::assert_unreachable!(
                                "BalanceShieldedTarget target doesn't exist.",
                                &json!({
                                    "target_alias": target,
                                    "pre_balance": pre_balance,
                                    "amount": amount,
                                    "timeout": random_timeout,
                                    "execution_height": execution_height,
                                    "check_height": latest_block
                                })
                            );
                            return Err(StepError::StateCheck(
                                "BalanceShieldedTarget check error: amount doesn't exist"
                                    .to_string(),
                            ));
                        }
                        Err(e) => {
                            return Err(StepError::StateCheck(format!(
                                "BalanceShieldedTarget check error: {e}"
                            )));
                        }
                    };
                }
                Check::BalanceSource(target, pre_balance, amount) => {
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
                            tracing::info!("Retry {attempt} due to {error}...");
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
                                return Err(StepError::StateCheck(
                                    "BalanceTarget check error: balance is negative".to_string(),
                                ));
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
                                        "timeout": random_timeout,
                                        "execution_height": execution_height,
                                        "check_height": latest_block
                                    })
                                );
                                return Err(StepError::StateCheck(format!("BalanceSource check error: post target amount not equal to pre balance: pre {pre_balance}, post: {post_amount}, {amount}")));
                            }
                        }
                        Err(e) => {
                            return Err(StepError::StateCheck(format!(
                                "BalanceSource check error: {e}"
                            )))
                        }
                    }
                }
                Check::BondIncrease(target, validator, pre_bond, amount) => {
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
                                return Err(StepError::StateCheck(
                                    "Bond increase check error: bond is negative".to_string(),
                                ));
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
                                        "epoch": epoch,
                                        "timeout": random_timeout,
                                        "execution_height": execution_height,
                                        "check_height": latest_block
                                    })
                                );
                                return Err(StepError::StateCheck(format!("Bond increase check error: post target amount is not equal to pre balance: pre {pre_bond}, post {post_bond}, amount: {amount}")));
                            }
                        }
                        Err(e) => {
                            return Err(StepError::StateCheck(format!("Bond check error: {e}")))
                        }
                    }
                }
                Check::BondDecrease(target, validator, pre_bond, amount) => {
                    let wallet = sdk.namada.wallet.read().await;
                    let source_address = wallet.find_address(&target.name).unwrap().into_owned();

                    let validator_address = Address::from_str(&validator).unwrap();

                    let epoch = if let Ok(epoch) = tryhard::retry_fn(|| rpc::query_epoch(&client))
                        .with_config(config)
                        .on_retry(|attempt, _, error| {
                            let error = error.to_string();
                            async move {
                                tracing::info!("Retry {attempt} due to {error}...");
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
                                return Err(StepError::StateCheck(
                                    "Bond decrease check error: bond is negative".to_string(),
                                ));
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
                                        "epoch": epoch,
                                        "timeout": random_timeout,
                                        "execution_height": execution_height,
                                        "check_height": latest_block
                                    })
                                );
                                return Err(StepError::StateCheck(format!("Bond decrease check error: post target amount is not equal to pre balance: pre {pre_bond}, post {post_bond}, amount: {amount}")));
                            }
                        }
                        Err(e) => {
                            return Err(StepError::StateCheck(format!("Bond check error: {e}")))
                        }
                    }
                }
                Check::AccountExist(target, threshold, sources) => {
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
                                        "timeout": random_timeout,
                                        "execution_height": execution_height,
                                        "check_height": latest_block
                                    })
                                );
                                return Err(StepError::StateCheck(format!(
                                    "AccountExist check error: account {} is invalid",
                                    source_address
                                )));
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
                                    "timeout": random_timeout,
                                    "execution_height": execution_height,
                                    "check_height": latest_block
                                })
                            );
                            return Err(StepError::StateCheck(format!(
                                "AccountExist check error: account {} doesn't exist",
                                target.name
                            )));
                        }
                        Err(e) => {
                            return Err(StepError::StateCheck(format!(
                                "AccountExist check error: {e}"
                            )))
                        }
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
                                tracing::info!("Retry {attempt} due to {error}...");
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
                            tracing::info!("Retry {attempt} due to {error}...");
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
                                crate::check::ValidatorStatus::Reactivating => {
                                    state.ne(&ValidatorState::Inactive)
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
                            return Err(StepError::StateCheck(format!(
                                "Validator status check error: validator {} doesn't exist",
                                target.name
                            )));
                        }
                        Err(e) => {
                            return Err(StepError::StateCheck(format!(
                                "ValidatorStatus check error: {e}"
                            )))
                        }
                    };
                }
            }
        }

        Ok(())
    }

    pub async fn execute(
        &self,
        sdk: &Sdk,
        tasks: &Vec<Task>,
    ) -> Result<Vec<ExecutionResult>, StepError> {
        let mut execution_results = vec![];

        for task in tasks {
            let now = Instant::now();
            let execution_height = task.execute(sdk).await?;
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
}
