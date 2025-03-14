use std::collections::HashMap;
use std::time::Instant;

use namada_sdk::rpc;
use tokio::time::{sleep, Duration};

use crate::check::{Check, CheckContext, CheckInfo};
use crate::error::{CheckError, StepError, TaskError};
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::step::{StepContext, StepType};
use crate::task::{Task, TaskContext};
use crate::types::{Alias, Epoch, Fee, Height};
use crate::utils::{execute_reveal_pk, is_pk_revealed, retry_config};

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

    async fn fetch_current_block_height(&self) -> Height {
        loop {
            if let Ok(Some(latest_block)) = rpc::query_block(&self.sdk.namada.client).await {
                return latest_block.height.into();
            }
            sleep(Duration::from_secs(1)).await
        }
    }

    async fn wait_block_settlement(&self, height: Height) {
        loop {
            let current_height = self.fetch_current_block_height().await;
            if current_height > height {
                break;
            } else {
                tracing::info!(
                    "Waiting for block settlement at height: {}, currently at: {}",
                    height,
                    current_height
                );
            }
            sleep(Duration::from_secs(2)).await
        }
    }

    async fn fetch_epoch_at_height(&self, height: Height) -> Epoch {
        loop {
            let epoch = rpc::query_epoch_at_height(&self.sdk.namada.client, height.into()).await;
            if let Ok(epoch) = epoch {
                return epoch.expect("Epoch should exist").into();
            }
            sleep(Duration::from_secs(1)).await
        }
    }

    pub async fn init(&self) -> Result<(), StepError> {
        let client = &self.sdk.namada.client;
        let wallet = self.sdk.namada.wallet.read().await;
        let faucet_alias = Alias::faucet();
        let faucet_address = wallet
            .find_address(&faucet_alias.name)
            .ok_or_else(|| StepError::Wallet(format!("No source address: {}", faucet_alias.name)))?
            .into_owned();
        let native_token_alias = Alias::nam();
        let nam_address = wallet
            .find_address(&native_token_alias.name)
            .ok_or_else(|| {
                StepError::Wallet(format!(
                    "No native token address: {}",
                    native_token_alias.name
                ))
            })?
            .into_owned();
        let faucet_public_key = wallet
            .find_public_key(&faucet_alias.name)
            .map_err(|e| StepError::Wallet(e.to_string()))?
            .to_owned();
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
            if let Ok(is_revealed) = is_pk_revealed(&self.sdk, &faucet_alias, retry_config()).await
            {
                if is_revealed {
                    break;
                }
            }
            if execute_reveal_pk(&self.sdk, faucet_public_key.clone())
                .await
                .is_ok()
            {
                break;
            }
            tracing::warn!("Retry revealing faucet pk...");
            sleep(Duration::from_secs(2)).await;
        }

        Ok(())
    }

    pub async fn is_valid(&self, step_type: &StepType) -> Result<bool, StepError> {
        step_type.is_valid(&self.sdk, &self.state).await
    }

    pub async fn build(&self, step_type: &StepType) -> Result<Vec<Task>, StepError> {
        step_type.build_task(&self.sdk, &self.state).await
    }

    pub async fn build_check(&self, tasks: &[Task]) -> Result<Vec<Check>, TaskError> {
        let retry_config = retry_config();
        let mut checks = vec![];
        for task in tasks {
            let built_checks = task.build_checks(&self.sdk, retry_config).await?;
            built_checks
                .iter()
                .for_each(|check| check.assert_pre_balance(&self.state));
            checks.extend(built_checks)
        }
        Ok(checks)
    }

    pub async fn checks(
        &self,
        checks: Vec<Check>,
        execution_height: Height,
        fees: &HashMap<Alias, Fee>,
    ) -> Result<(), CheckError> {
        let retry_config = retry_config();

        if checks.is_empty() {
            return Ok(());
        }

        let check_height = self.fetch_current_block_height().await;
        for check in checks {
            tracing::info!("Running {check} check...");
            check
                .do_check(
                    &self.sdk,
                    fees,
                    CheckInfo {
                        execution_height,
                        check_height,
                    },
                    retry_config,
                )
                .await?;
        }

        Ok(())
    }

    pub async fn execute(
        &self,
        tasks: &[Task],
    ) -> (Result<Height, TaskError>, HashMap<Alias, Fee>) {
        let mut total_time = 0;
        let mut heights = vec![];
        let mut fees = HashMap::new();

        let start_height = self.fetch_current_block_height().await;

        for task in tasks {
            tracing::info!("Executing {task}...");
            let now = Instant::now();
            let execution_height = match task.execute(&self.sdk).await {
                Ok(height) => height,
                Err(e) => {
                    match e {
                        // aggreate fees when the tx has been executed
                        TaskError::Execution(_) => task.aggregate_fees(&mut fees),
                        TaskError::Broadcast(_) => self.wait_block_settlement(start_height).await,
                        _ => {}
                    }
                    return (Err(e), fees);
                }
            };

            total_time += now.elapsed().as_secs();
            heights.push(execution_height);

            task.aggregate_fees(&mut fees);
        }
        tracing::info!("Execution took {total_time}s...");

        let execution_height = heights.into_iter().max().unwrap_or_default();
        // wait for the execution block settling
        self.wait_block_settlement(execution_height).await;

        (Ok(execution_height), fees)
    }

    pub async fn post_execute(
        &mut self,
        tasks: &[Task],
        execution_height: Height,
    ) -> Result<(), TaskError> {
        for task in tasks {
            // update state
            task.update_state(&mut self.state);
            task.update_stats(&mut self.state);

            match task {
                Task::ClaimRewards(cr) => {
                    // workaround for exact balance update after claim-rewards
                    let (_, balance) = crate::utils::get_balance(
                        &self.sdk,
                        cr.source(),
                        crate::utils::retry_config(),
                    )
                    .await?;
                    let balance = balance
                        .to_string()
                        .parse()
                        .expect("Balance conversion shouldn't fail");
                    self.state.overwrite_balance(cr.source(), balance);

                    let claimed_epoch = self.fetch_epoch_at_height(execution_height).await;
                    self.state.set_claimed_epoch(cr.source(), claimed_epoch);
                }
                Task::InitAccount(_) => {
                    // save wallet for init-account
                    let wallet = self.sdk.namada.wallet.read().await;
                    wallet
                        .save()
                        .map_err(|e| TaskError::Wallet(e.to_string()))?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn apply_fee_payments(&mut self, fees: &HashMap<Alias, Fee>) {
        fees.iter()
            .for_each(|(payer, fee)| self.state.modify_balance_fee(payer, *fee));
    }
}
