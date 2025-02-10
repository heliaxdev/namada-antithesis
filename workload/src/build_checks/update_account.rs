use std::collections::BTreeSet;

use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};

use crate::{check::Check, entities::Alias, sdk::namada::Sdk, state::State};

pub async fn update_account_build_checks(
    _sdk: &Sdk,
    alias: Alias,
    sources: BTreeSet<Alias>,
    threshold: u64,
    _retry_config: RetryFutureConfig<ExponentialBackoff, NoOnRetry>,
    state: &State,
) -> Vec<Check> {
    vec![Check::AccountExist(
        alias,
        threshold,
        sources,
        state.clone(),
    )]
}
