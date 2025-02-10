use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};

use crate::{
    check::{Check, ValidatorStatus},
    entities::Alias,
    sdk::namada::Sdk,
    state::State,
};

pub async fn deactivate_validator_build_checks(
    _sdk: &Sdk,
    alias: Alias,
    _retry_config: RetryFutureConfig<ExponentialBackoff, NoOnRetry>,
    _state: &State,
) -> Vec<Check> {
    vec![Check::ValidatorStatus(alias, ValidatorStatus::Inactive)]
}
