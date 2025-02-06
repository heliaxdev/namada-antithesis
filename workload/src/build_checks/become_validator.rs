use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};

use crate::{check::Check, entities::Alias, sdk::namada::Sdk, state::State};

pub async fn become_validator(
    source: Alias,
) -> Vec<Check> {
    vec![Check::IsValidatorAccount(source)]
}
