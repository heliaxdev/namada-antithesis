use std::collections::{BTreeSet, HashMap};
use std::fmt::Display;

use enum_dispatch::enum_dispatch;
use namada_sdk::{args, signing::SigningTxData, tx::Tx};

use crate::check::Check;
use crate::constants::DEFAULT_GAS_LIMIT;
use crate::context::Ctx;
use crate::error::TaskError;
use crate::state::State;
use crate::types::{Alias, Fee, Height, MaspEpoch};
use crate::utils;
use crate::utils::RetryConfig;

pub mod batch;
pub mod become_validator;
pub mod bond;
pub mod change_consensus_key;
pub mod change_metadata;
pub mod claim_rewards;
pub mod deactivate_validator;
pub mod default_proposal;
pub mod faucet_transfer;
pub mod init_account;
pub mod new_wallet_keypair;
pub mod reactivate_validator;
pub mod redelegate;
pub mod shielded;
pub mod shielding;
pub mod transparent_transfer;
pub mod unbond;
pub mod unshielding;
pub mod update_account;
pub mod vote;

#[derive(Clone, Debug)]
pub struct TaskSettings {
    pub signers: BTreeSet<Alias>,
    pub gas_payer: Alias,
    pub gas_limit: u64,
}

impl TaskSettings {
    pub fn new(signers: BTreeSet<Alias>, gas_payer: Alias) -> Self {
        Self {
            signers,
            gas_payer,
            gas_limit: DEFAULT_GAS_LIMIT,
        }
    }

    pub fn faucet() -> Self {
        Self {
            signers: BTreeSet::from_iter(vec![Alias::faucet()]),
            gas_payer: Alias::faucet(),
            gas_limit: DEFAULT_GAS_LIMIT,
        }
    }

    pub fn faucet_batch(size: usize) -> Self {
        Self {
            signers: BTreeSet::from_iter(vec![Alias::faucet()]),
            gas_payer: Alias::faucet(),
            gas_limit: DEFAULT_GAS_LIMIT * size as u64,
        }
    }
}

#[enum_dispatch]
#[derive(Clone, Debug)]
pub enum Task {
    NewWalletKeyPair(new_wallet_keypair::NewWalletKeyPair),
    FaucetTransfer(faucet_transfer::FaucetTransfer),
    TransparentTransfer(transparent_transfer::TransparentTransfer),
    Bond(bond::Bond),
    Unbond(unbond::Unbond),
    Redelegate(redelegate::Redelegate),
    ClaimRewards(claim_rewards::ClaimRewards),
    Batch(batch::Batch),
    ShieldedTransfer(shielded::ShieldedTransfer),
    Shielding(shielding::Shielding),
    InitAccount(init_account::InitAccount),
    Unshielding(unshielding::Unshielding),
    BecomeValidator(become_validator::BecomeValidator),
    ChangeMetadata(change_metadata::ChangeMetadata),
    ChangeConsensusKey(change_consensus_key::ChangeConsensusKey),
    DeactivateValidator(deactivate_validator::DeactivateValidator),
    ReactivateValidator(reactivate_validator::ReactivateValidator),
    UpdateAccount(update_account::UpdateAccount),
    DefaultProposal(default_proposal::DefaultProposal),
    Vote(vote::Vote),
}

impl Task {
    pub fn aggregate_fees(&self, fees: &mut HashMap<Alias, Fee>, is_successful: bool) {
        match self {
            Task::Batch(batch) => {
                let tasks = batch.tasks();
                if tasks.len() == 1 {
                    let task = tasks.first().expect("Task should exist");
                    if let Some(settings) = task.task_settings() {
                        *fees.entry(settings.gas_payer.clone()).or_insert(0) += settings.gas_limit;
                    }
                } else {
                    if is_successful {
                        tasks
                            .iter()
                            .filter(|task| {
                                matches!(task, Task::ShieldedTransfer(_) | Task::Unshielding(_))
                            })
                            .for_each(|task| {
                                let settings = task
                                    .task_settings()
                                    .expect("Shielded task should have settings");
                                let gas_payer = &settings.gas_payer;
                                if gas_payer.is_spending_key() {
                                    *fees.entry(gas_payer.clone()).or_insert(0) +=
                                        settings.gas_limit;
                                }
                            });
                    }
                    // fee for wrapper tx
                    let settings = batch.task_settings().expect("TaskSettings should exist");
                    *fees.entry(settings.gas_payer.clone()).or_insert(0) += settings.gas_limit;
                }
            }
            _ => {
                if let Some(settings) = self.task_settings() {
                    *fees.entry(settings.gas_payer.clone()).or_insert(0) += settings.gas_limit;
                }
            }
        }
    }
}

impl Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

#[enum_dispatch(Task)]
pub trait TaskContext {
    fn name(&self) -> String;

    fn summary(&self) -> String;

    fn task_settings(&self) -> Option<&TaskSettings>;

    #[allow(async_fn_in_trait)]
    async fn build_tx(&self, ctx: &Ctx) -> Result<(Tx, Vec<SigningTxData>, args::Tx), TaskError>;

    #[allow(async_fn_in_trait)]
    async fn execute(&self, ctx: &Ctx) -> Result<Height, TaskError> {
        let (tx, signing_data, tx_args) = self.build_tx(ctx).await?;
        utils::execute_tx(ctx, tx, signing_data, &tx_args).await
    }

    #[allow(async_fn_in_trait)]
    async fn execute_shielded_tx(
        &self,
        ctx: &Ctx,
        start_epoch: MaspEpoch,
    ) -> Result<Height, TaskError> {
        let retry_config = utils::retry_config();

        let height = utils::get_block_height(ctx, retry_config).await?;
        let result = match self.build_tx(ctx).await {
            Ok((tx, signing_data, tx_args)) => {
                utils::execute_tx(ctx, tx, signing_data, &tx_args).await
            }
            Err(e) => Err(e),
        };

        let epoch = match result {
            Ok(_) => None,
            Err(TaskError::Execution { height, .. })
            | Err(TaskError::InsufficientGas { height, .. }) => {
                utils::wait_block_settlement(ctx, height, retry_config).await;
                Some(utils::get_masp_epoch_at_height(ctx, height, retry_config).await?)
            }
            Err(_) => {
                utils::wait_block_settlement(ctx, height + 1, retry_config).await;
                Some(utils::get_masp_epoch(ctx, retry_config).await?)
            }
        };

        result.map_err(|err| {
            if epoch == Some(start_epoch) {
                err
            } else {
                TaskError::InvalidShielded {
                    err: err.to_string(),
                    was_fee_paid: matches!(err, TaskError::Execution { .. }),
                }
            }
        })
    }

    #[allow(async_fn_in_trait)]
    async fn build_checks(
        &self,
        ctx: &Ctx,
        retry_config: RetryConfig,
    ) -> Result<Vec<Check>, TaskError>;

    fn update_state(&self, state: &mut State);

    fn update_stats(&self, state: &mut State) {
        state
            .stats
            .entry(self.name())
            .and_modify(|counter| *counter += 1)
            .or_insert(1);
    }
}
