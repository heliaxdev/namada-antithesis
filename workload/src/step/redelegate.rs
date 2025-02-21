use std::str::FromStr;

use namada_sdk::{address::Address, rpc};
use rand::seq::IteratorRandom;

use crate::{
    entities::Alias,
    executor::StepError,
    sdk::namada::Sdk,
    state::State,
    step::StepContext,
    task::{self, Task, TaskSettings},
};

use super::utils;

#[derive(Debug, Default)]
pub struct Redelegate;

impl StepContext for Redelegate {
    fn name(&self) -> String {
        "redelegate".to_string()
    }

    async fn is_valid(&self, _sdk: &Sdk, state: &State) -> Result<bool, StepError> {
        Ok(state.any_bond())
    }

    async fn build_task(&self, sdk: &Sdk, state: &mut State) -> Result<Vec<Task>, StepError> {
        let client = &sdk.namada.client;
        let source_bond = state.random_bond();
        let source_account = state.get_account_by_alias(&source_bond.alias);
        let amount = utils::random_between(state, 1, source_bond.amount);

        let current_epoch = rpc::query_epoch(client).await.map_err(StepError::Rpc)?;
        let validators = rpc::get_all_consensus_validators(client, current_epoch)
            .await
            .map_err(StepError::Rpc)?;

        let source_bond_validator_address = Address::from_str(&source_bond.validator)
            .expect("ValidatorAddress should be converted");

        let source_redelegations = state.get_redelegations_targets_for(&source_account.alias);
        if source_redelegations.contains(&source_bond.validator) {
            return Ok(vec![]);
        }

        let to_validator = if let Some(validator) = validators
            .into_iter()
            .filter_map(|v| {
                if v.address.eq(&source_bond_validator_address) {
                    None
                } else {
                    Some(v.address)
                }
            })
            .choose(&mut state.rng)
        {
            validator
        } else {
            return Ok(vec![]);
        };

        let mut task_settings = TaskSettings::new(source_account.public_keys, Alias::faucet());
        task_settings.gas_limit *= 5;

        Ok(vec![Task::Redelegate(
            task::redelegate::Redelegate::builder()
                .source(source_account.alias)
                .from_validator(source_bond.validator.to_string())
                .to_validator(to_validator.to_string())
                .amount(amount)
                .epoch(
                    current_epoch
                        .checked_add(6)
                        .expect("Epoch shouldn't overflow")
                        .into(),
                )
                .settings(task_settings)
                .build(),
        )])
    }
}
