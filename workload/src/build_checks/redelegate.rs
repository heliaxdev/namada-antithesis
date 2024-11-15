use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};

use crate::{check::Check, entities::Alias, sdk::namada::Sdk, state::State};

pub async fn redelegate(
    sdk: &Sdk,
    source: Alias,
    from_validator: String,
    to_validator: String,
    amount: u64,
    epoch: u64,
    retry_config: RetryFutureConfig<ExponentialBackoff, NoOnRetry>,
    state: &State,
) -> Vec<Check> {
    let from_validator_bond_check = if let Some(pre_bond) = super::utils::get_bond(
        sdk,
        source.clone(),
        from_validator.clone(),
        epoch,
        retry_config,
    )
    .await
    {
        Check::BondDecrease(
            source.clone(),
            from_validator,
            pre_bond,
            amount,
            state.clone(),
        )
    } else {
        return vec![];
    };
    let to_validator_bond_check = if let Some(pre_bond) = super::utils::get_bond(
        sdk,
        source.clone(),
        to_validator.clone(),
        epoch,
        retry_config,
    )
    .await
    {
        Check::BondIncrease(source, to_validator, pre_bond, amount, state.clone())
    } else {
        return vec![];
    };
    vec![from_validator_bond_check, to_validator_bond_check]
}
