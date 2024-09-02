use namada_sdk::{
    args::{InputAmount, TxBuilder, TxTransparentTransferData},
    key::SchemeType,
    signing::default_sign,
    token::{self, DenominatedAmount},
    tx::{data::GasLimit, either, ProcessTxResponse, Tx},
    Namada,
};
use rand::{
    distributions::{Alphanumeric, DistString},
    rngs::OsRng,
    Rng,
};
use thiserror::Error;
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
}

use crate::{
    entities::Alias,
    sdk::namada::Sdk,
    state::State,
    task::{Source, Target, Task, TaskSettings},
};

#[derive(Clone, Debug, Copy)]
pub enum StepType {
    NewWalletKeyPair,
    FaucetTransfer,
    TransparentTransfer,
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
                state.at_least_accounts(2)
                    && state.any_account_can_pay_fees()
                    && state.any_account_can_make_transfer()
            }
        }
    }

    pub fn build(&self, step_type: StepType, state: &State) -> Vec<Task> {
        match step_type {
            StepType::NewWalletKeyPair => {
                let alias = Self::random_alias();
                vec![Task::NewWalletKeyPair(alias)]
            }
            StepType::FaucetTransfer => {
                let target_account = state.random_account();
                let amount = Self::random_between(1000, 2000);

                let task_settings = TaskSettings::new(vec![Alias::faucet()], Alias::faucet());

                vec![Task::FaucetTransfer(
                    target_account.alias,
                    amount,
                    task_settings,
                )]
            }
            StepType::TransparentTransfer => {
                let source_account = state.random_account_with_min_balance(vec![]);
                let target_account =
                    state.random_account_with_min_balance(vec![source_account.alias.clone()]);
                let amount = state.get_balance_for(&source_account.alias);

                let task_settings = TaskSettings::new(vec![Alias::faucet()], Alias::faucet());

                vec![Task::TransparentTransfer(
                    source_account.alias,
                    target_account.alias,
                    amount,
                    task_settings,
                )]
            }
        }
    }

    pub async fn execute(&self, sdk: &Sdk, tasks: Vec<Task>) -> Result<(), StepError> {
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

                    if let Some((alias, sk)) = keypair {
                        wallet.save().expect("unable to save wallet");
                        (alias, sk)
                    } else {
                        return Err(StepError::Wallet("Failed to save keypair".to_string()));
                    };
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
            }
        }
        Ok(())
    }

    pub fn update_state(&self, tasks: Vec<Task>, state: &mut State) {
        for task in tasks {
            match task {
                Task::NewWalletKeyPair(alias) => {
                    state.add_implicit_account(alias);
                },
                Task::FaucetTransfer(target, amount, settings) => {
                    let source_alias = Alias::faucet();
                    state.modify_balance(source_alias, target, amount);
                    state.modify_balance_fee(settings.gas_payer, settings.gas_limit);
                },
                Task::TransparentTransfer(source, target, amount, setting) => {
                    state.modify_balance(source, target, amount);
                    state.modify_balance_fee(setting.gas_payer, setting.gas_limit);
                },
            }
        }
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
                    tracing::debug!("batch result: {:#?}", batch);
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
}
