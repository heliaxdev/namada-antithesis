use std::{str::FromStr, time::Instant};

use namada_sdk::{
    address::Address,
    args::{InputAmount, TxBuilder, TxTransparentTransferData},
    key::{common, SchemeType},
    rpc,
    signing::default_sign,
    token::{self, DenominatedAmount},
    tx::{data::GasLimit, either, ProcessTxResponse, Tx},
    Namada,
};
use rand::{
    distributions::{Alphanumeric, DistString},
    rngs::OsRng,
    seq::IteratorRandom,
    Rng,
};
use serde_json::json;
use thiserror::Error;
use tokio::time::{sleep, Duration};
use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};
use weighted_rand::{
    builder::{NewBuilder, WalkerTableBuilder},
    table::WalkerTable,
};

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
    check::Check,
    entities::Alias,
    sdk::namada::Sdk,
    state::State,
    task::{Task, TaskSettings},
};

#[derive(Clone, Debug, Copy)]
pub enum StepType {
    NewWalletKeyPair,
    FaucetTransfer,
    TransparentTransfer,
    Bond,
}

#[derive(Clone, Debug)]
pub struct WorkloadExecutor {
    pub step_types: Vec<StepType>,
    inner: WalkerTable,
}

impl WorkloadExecutor {
    pub fn new(step_types: Vec<StepType>, step_prob: Vec<f32>) -> Self {
        let builder = WalkerTableBuilder::new(&step_prob);
        let table = builder.build();

        Self {
            step_types,
            inner: table,
        }
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

    pub fn next(&self, state: &State) -> StepType {
        let mut next_step = self.step_types[self.inner.next()];
        loop {
            if Self::is_valid(next_step, state) {
                return next_step;
            }
            next_step = self.step_types[self.inner.next()];
        }
    }

    fn is_valid(step_type: StepType, state: &State) -> bool {
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
        state: &State,
    ) -> Result<Vec<Task>, StepError> {
        let steps = match step_type {
            StepType::NewWalletKeyPair => {
                let alias = Self::random_alias();
                vec![Task::NewWalletKeyPair(alias)]
            }
            StepType::FaucetTransfer => {
                let target_account = state.random_account(vec![]);
                let amount = Self::random_between(1000, 2000);

                let task_settings = TaskSettings::faucet();

                vec![Task::FaucetTransfer(
                    target_account.alias,
                    amount,
                    task_settings,
                )]
            }
            StepType::TransparentTransfer => {
                let source_account = state.random_account_with_min_balance(vec![]);
                let target_account = state.random_account(vec![source_account.alias.clone()]);
                let amount = state.get_balance_for(&source_account.alias);

                let task_settings = TaskSettings::new(source_account.public_keys, Alias::faucet());

                vec![Task::TransparentTransfer(
                    source_account.alias,
                    target_account.alias,
                    amount,
                    task_settings,
                )]
            }
            StepType::Bond => {
                let client = sdk.namada.client();
                let source_account = state.random_account_with_min_balance(vec![]);
                let amount = state.get_balance_for(&source_account.alias);

                let current_epoch = rpc::query_epoch(client)
                    .await
                    .map_err(|e| StepError::Rpc(format!("query epoch: {}", e)))?;
                let validators = rpc::get_all_consensus_validators(client, current_epoch)
                    .await
                    .map_err(|e| StepError::Rpc(format!("query consensus validators: {}", e)))?;

                let validator = validators
                    .into_iter()
                    .map(|v| v.address)
                    .choose(&mut rand::thread_rng())
                    .unwrap(); // safe as there is always at least a validator

                let task_settings = TaskSettings::new(source_account.public_keys, Alias::faucet());

                vec![Task::Bond(
                    source_account.alias,
                    validator.to_string(),
                    amount,
                    current_epoch.into(),
                    task_settings,
                )]
            }
        };
        Ok(steps)
    }

    pub async fn build_check(&self, sdk: &Sdk, tasks: Vec<Task>) -> Vec<Check> {
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
                        rpc::get_token_balance(client, &native_token_address, &target_address)
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
                        Check::BalanceTarget(target, pre_balance, amount)
                    } else {
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
                        rpc::get_token_balance(client, &native_token_address, &source_address)
                    })
                    .with_config(config)
                    .await
                    {
                        Check::BalanceSource(source, pre_balance, amount)
                    } else {
                        continue;
                    };

                    let target_check = if let Ok(pre_balance) = tryhard::retry_fn(|| {
                        rpc::get_token_balance(client, &native_token_address, &target_address)
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
                        Check::BalanceTarget(target, pre_balance, amount)
                    } else {
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
                        rpc::get_bond_amount_at(client, &source_address, &validator_address, epoch)
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
                        Check::Bond(source, validator, pre_bond, amount)
                    } else {
                        continue;
                    };
                    vec![bond_check]
                }
            };
            checks.extend(check)
        }
        checks
    }

    pub async fn checks(&self, sdk: &Sdk, checks: Vec<Check>) -> Result<(), String> {
        let config = Self::retry_config();
        let client = sdk.namada.client();

        if checks.is_empty() {
            return Ok(());
        }

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
                            if !was_pk_revealed {
                                antithesis_sdk::assert_always!(
                                    was_pk_revealed,
                                    "The public key was not released correctly.",
                                    &json!({
                                        "public-key": source.to_pretty_string()
                                    })
                                );
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
                Check::BalanceTarget(target, pre_balance, amount) => {
                    let wallet = sdk.namada.wallet.read().await;
                    let native_token_address = wallet.find_address("nam").unwrap().into_owned();
                    let target_address = wallet.find_address(&target.name).unwrap().into_owned();
                    drop(wallet);

                    match tryhard::retry_fn(|| {
                        rpc::get_token_balance(client, &native_token_address, &target_address)
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
                            if !post_amount.le(&check_balance) {
                                antithesis_sdk::assert_always!(
                                    post_amount.le(&check_balance),
                                    "Balance target didn't increase.",
                                    &json!({
                                        "target": target_address.to_pretty_string(),
                                        "pre_balance": pre_balance,
                                        "amount": check_balance,
                                        "post_balance": post_amount
                                    })
                                );
                                return Err("BalanceTarget check error: post target amount is greater than pre balance".to_string());
                            }
                        }
                        Err(e) => return Err(format!("BalanceTarget check error: {}", e)),
                    }
                }
                Check::BalanceSource(target, pre_balance, amount) => {
                    let wallet = sdk.namada.wallet.read().await;
                    let native_token_address = wallet.find_address("nam").unwrap().into_owned();
                    let target_address = wallet.find_address(&target.name).unwrap().into_owned();
                    drop(wallet);

                    match tryhard::retry_fn(|| {
                        rpc::get_token_balance(client, &native_token_address, &target_address)
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
                            if !post_amount.ge(&check_balance) {
                                antithesis_sdk::assert_always!(
                                    post_amount.ge(&check_balance),
                                    "Balance source didn't decrease.",
                                    &json!({
                                        "target": target_address.to_pretty_string(),
                                        "pre_balance": pre_balance,
                                        "amount": check_balance,
                                        "post_balance": post_amount
                                    })
                                );
                                return Err(format!("BalanceTarget check error: post target amount is less than pre balance: pre {}, post: {}, {}", pre_balance, post_amount, amount));
                            }
                        }
                        Err(e) => return Err(format!("BalanceTarget check error: {}", e)),
                    }
                }
                Check::Bond(target, validator, pre_bond, amount) => {
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
                        rpc::get_bond_amount_at(client, &source_address, &validator_address, epoch)
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
                                post_bond.ge(&check_bond),
                                "Bond source didn't increase invalid.",
                                &json!({
                                    "target": source_address.to_pretty_string(),
                                    "validator": validator_address.to_pretty_string(),
                                    "pre_bond": pre_bond,
                                    "amount": amount,
                                    "post_bond": post_bond
                                })
                            );
                            if !post_bond.le(&check_bond) {
                                return Err(format!("Bond check error: post target amount is less than pre balance: pre {}, post {}, amount: {}", pre_bond, post_bond, amount));
                            }
                        }
                        Err(e) => return Err(format!("Bond check error: {}", e)),
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn execute(&self, sdk: &Sdk, tasks: Vec<Task>) -> Result<u64, StepError> {
        let now = Instant::now();

        for task in tasks {
            match task {
                Task::NewWalletKeyPair(alias) => {
                    let mut wallet = sdk.namada.wallet.write().await;

                    let keypair = wallet.gen_store_secret_key(
                        SchemeType::Ed25519,
                        Some(alias.name),
                        true,
                        None,
                        &mut OsRng,
                    );

                    let (_alias, sk) = if let Some((alias, sk)) = keypair {
                        wallet.save().expect("unable to save wallet");
                        (alias, sk)
                    } else {
                        return Err(StepError::Wallet("Failed to save keypair".to_string()));
                    };
                    drop(wallet);

                    let public_key = sk.to_public();
                    Self::reveal_pk(sdk, public_key).await?
                }
                Task::FaucetTransfer(target, amount, settings) => {
                    let wallet = sdk.namada.wallet.write().await;

                    let faucet_alias = Alias::faucet();
                    let native_token_alias = Alias::nam();

                    let source_address = wallet
                        .find_address(faucet_alias.name)
                        .unwrap()
                        .as_ref()
                        .clone();
                    let target_address = wallet.find_address(target.name).unwrap().as_ref().clone();
                    let token_address = wallet
                        .find_address(native_token_alias.name)
                        .unwrap()
                        .as_ref()
                        .clone();
                    let fee_payer = wallet.find_public_key(&settings.gas_payer.name).unwrap();
                    let token_amount = token::Amount::from_u64(amount);

                    let tx_transfer_data = TxTransparentTransferData {
                        source: source_address.clone(),
                        target: target_address.clone(),
                        token: token_address,
                        amount: InputAmount::Unvalidated(DenominatedAmount::native(token_amount)),
                    };

                    let mut transfer_tx_builder =
                        sdk.namada.new_transparent_transfer(vec![tx_transfer_data]);

                    transfer_tx_builder =
                        transfer_tx_builder.gas_limit(GasLimit::from(settings.gas_limit));
                    transfer_tx_builder = transfer_tx_builder.wrapper_fee_payer(fee_payer);

                    let mut signing_keys = vec![];
                    for signer in settings.signers {
                        let public_key = wallet.find_public_key(&signer.name).unwrap();
                        signing_keys.push(public_key)
                    }
                    transfer_tx_builder = transfer_tx_builder.signing_keys(signing_keys.clone());
                    drop(wallet);

                    let (mut transfer_tx, signing_data) = transfer_tx_builder
                        .build(&sdk.namada)
                        .await
                        .map_err(|e| StepError::Build(e.to_string()))?;

                    sdk.namada
                        .sign(
                            &mut transfer_tx,
                            &transfer_tx_builder.tx,
                            signing_data,
                            default_sign,
                            (),
                        )
                        .await
                        .expect("unable to sign tx");

                    let tx = sdk
                        .namada
                        .submit(transfer_tx.clone(), &transfer_tx_builder.tx)
                        .await;

                    if Self::is_tx_rejected(&transfer_tx, &tx) {
                        match tx {
                            Ok(tx) => {
                                let errors =
                                    Self::get_tx_errors(&transfer_tx, &tx).unwrap_or_default();
                                return Err(StepError::Execution(errors));
                            }
                            Err(e) => return Err(StepError::Broadcast(e.to_string())),
                        }
                    }
                }
                Task::TransparentTransfer(source, target, amount, settings) => {
                    let wallet = sdk.namada.wallet.write().await;

                    let native_token_alias = Alias::nam();

                    let source_address = wallet.find_address(source.name).unwrap().as_ref().clone();
                    let target_address = wallet.find_address(target.name).unwrap().as_ref().clone();
                    let token_address = wallet
                        .find_address(native_token_alias.name)
                        .unwrap()
                        .as_ref()
                        .clone();
                    let fee_payer = wallet.find_public_key(&settings.gas_payer.name).unwrap();
                    let token_amount = token::Amount::from_u64(amount);

                    let tx_transfer_data = TxTransparentTransferData {
                        source: source_address.clone(),
                        target: target_address.clone(),
                        token: token_address,
                        amount: InputAmount::Unvalidated(DenominatedAmount::native(token_amount)),
                    };

                    let mut transfer_tx_builder =
                        sdk.namada.new_transparent_transfer(vec![tx_transfer_data]);
                    transfer_tx_builder =
                        transfer_tx_builder.gas_limit(GasLimit::from(settings.gas_limit));
                    transfer_tx_builder = transfer_tx_builder.wrapper_fee_payer(fee_payer);
                    let mut signing_keys = vec![];
                    for signer in settings.signers {
                        let public_key = wallet.find_public_key(&signer.name).unwrap();
                        signing_keys.push(public_key)
                    }
                    transfer_tx_builder = transfer_tx_builder.signing_keys(signing_keys.clone());
                    drop(wallet);

                    let (mut transfer_tx, signing_data) = transfer_tx_builder
                        .build(&sdk.namada)
                        .await
                        .map_err(|e| StepError::Build(e.to_string()))?;

                    sdk.namada
                        .sign(
                            &mut transfer_tx,
                            &transfer_tx_builder.tx,
                            signing_data,
                            default_sign,
                            (),
                        )
                        .await
                        .expect("unable to sign tx");

                    let tx = sdk
                        .namada
                        .submit(transfer_tx.clone(), &transfer_tx_builder.tx)
                        .await;

                    if Self::is_tx_rejected(&transfer_tx, &tx) {
                        match tx {
                            Ok(tx) => {
                                let errors =
                                    Self::get_tx_errors(&transfer_tx, &tx).unwrap_or_default();
                                return Err(StepError::Execution(errors));
                            }
                            Err(e) => return Err(StepError::Broadcast(e.to_string())),
                        }
                    }
                }
                Task::Bond(source, validator, amount, _, settings) => {
                    let wallet = sdk.namada.wallet.write().await;

                    let source_address = wallet.find_address(source.name).unwrap().as_ref().clone();
                    let token_amount = token::Amount::from_u64(amount);
                    let fee_payer = wallet.find_public_key(&settings.gas_payer.name).unwrap();
                    let validator = Address::from_str(&validator).unwrap(); // safe

                    let mut bond_tx_builder = sdk
                        .namada
                        .new_bond(validator, token_amount)
                        .source(source_address);
                    bond_tx_builder = bond_tx_builder.gas_limit(GasLimit::from(settings.gas_limit));
                    bond_tx_builder = bond_tx_builder.wrapper_fee_payer(fee_payer);
                    let mut signing_keys = vec![];
                    for signer in settings.signers {
                        let public_key = wallet.find_public_key(&signer.name).unwrap();
                        signing_keys.push(public_key)
                    }
                    bond_tx_builder = bond_tx_builder.signing_keys(signing_keys.clone());
                    drop(wallet);

                    let (mut bond_tx, signing_data) = bond_tx_builder
                        .build(&sdk.namada)
                        .await
                        .map_err(|e| StepError::Build(e.to_string()))?;

                    sdk.namada
                        .sign(
                            &mut bond_tx,
                            &bond_tx_builder.tx,
                            signing_data,
                            default_sign,
                            (),
                        )
                        .await
                        .expect("unable to sign tx");

                    let tx = sdk
                        .namada
                        .submit(bond_tx.clone(), &bond_tx_builder.tx)
                        .await;

                    if Self::is_tx_rejected(&bond_tx, &tx) {
                        match tx {
                            Ok(tx) => {
                                let errors = Self::get_tx_errors(&bond_tx, &tx).unwrap_or_default();
                                return Err(StepError::Execution(errors));
                            }
                            Err(e) => return Err(StepError::Broadcast(e.to_string())),
                        }
                    }
                }
            }
        }
        Ok(now.elapsed().as_secs())
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

    async fn reveal_pk(sdk: &Sdk, public_key: common::PublicKey) -> Result<(), StepError> {
        let wallet = sdk.namada.wallet.write().await;
        let fee_payer = wallet.find_public_key("faucet").unwrap();
        drop(wallet);

        let reveal_pk_tx_builder = sdk
            .namada
            .new_reveal_pk(public_key.clone())
            .signing_keys(vec![public_key.clone()])
            .wrapper_fee_payer(fee_payer);

        let (mut reveal_tx, signing_data) = reveal_pk_tx_builder
            .build(&sdk.namada)
            .await
            .map_err(|e| StepError::Build(e.to_string()))?;

        sdk.namada
            .sign(
                &mut reveal_tx,
                &reveal_pk_tx_builder.tx,
                signing_data,
                default_sign,
                (),
            )
            .await
            .expect("unable to sign tx");

        let tx = sdk
            .namada
            .submit(reveal_tx.clone(), &reveal_pk_tx_builder.tx)
            .await;

        if Self::is_tx_rejected(&reveal_tx, &tx) {
            match tx {
                Ok(tx) => {
                    let errors = Self::get_tx_errors(&reveal_tx, &tx).unwrap_or_default();
                    return Err(StepError::Execution(errors));
                }
                Err(e) => return Err(StepError::Broadcast(e.to_string())),
            }
        }

        Ok(())
    }

    fn random_alias() -> Alias {
        format!(
            "load-tester-{}",
            Alphanumeric.sample_string(&mut rand::thread_rng(), 8)
        )
        .into()
    }

    fn random_between(from: u64, to: u64) -> u64 {
        rand::thread_rng().gen_range(from..to)
    }

    fn is_tx_rejected(
        tx: &Tx,
        tx_response: &Result<ProcessTxResponse, namada_sdk::error::Error>,
    ) -> bool {
        let cmt = tx.first_commitments().unwrap().to_owned();
        let wrapper_hash = tx.wrapper_hash();
        match tx_response {
            Ok(tx_result) => tx_result
                .is_applied_and_valid(wrapper_hash.as_ref(), &cmt)
                .is_none(),
            Err(_) => true,
        }
    }

    fn get_tx_errors(tx: &Tx, tx_response: &ProcessTxResponse) -> Option<String> {
        let cmt = tx.first_commitments().unwrap().to_owned();
        let wrapper_hash = tx.wrapper_hash();
        match tx_response {
            ProcessTxResponse::Applied(result) => match &result.batch {
                Some(batch) => {
                    tracing::info!("batch result: {:#?}", batch);
                    match batch.get_inner_tx_result(wrapper_hash.as_ref(), either::Right(&cmt)) {
                        Some(Ok(res)) => {
                            let errors = res.vps_result.errors.clone();
                            let _status_flag = res.vps_result.status_flags;
                            let _rejected_vps = res.vps_result.rejected_vps.clone();
                            Some(serde_json::to_string(&errors).unwrap())
                        }
                        Some(Err(e)) => Some(e.to_string()),
                        _ => None,
                    }
                }
                None => None,
            },
            _ => None,
        }
    }

    fn retry_config() -> RetryFutureConfig<ExponentialBackoff, NoOnRetry> {
        RetryFutureConfig::new(4)
            .exponential_backoff(Duration::from_secs(1))
            .max_delay(Duration::from_secs(10))
    }
}
