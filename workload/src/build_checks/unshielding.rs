use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};

use crate::{check::Check, entities::Alias, sdk::namada::Sdk, state::State};

pub async fn unshielding(
    sdk: &Sdk,
    source: Alias,
    target: Alias,
    amount: u64,
    with_indexer: bool,
    retry_config: RetryFutureConfig<ExponentialBackoff, NoOnRetry>,
    state: &State,
) -> Vec<Check> {
    let source_check = if let Ok(Some(pre_balance)) =
        super::utils::get_shielded_balance(sdk, source.clone(), None, with_indexer).await
    {
        Check::BalanceShieldedSource(source, pre_balance, amount, state.clone())
    } else {
        return vec![];
    };

    let target_check = if let Some(pre_balance) =
        super::utils::get_balance(sdk, target.clone(), retry_config).await
    {
        Check::BalanceTarget(target, pre_balance, amount, state.clone())
    } else {
        return vec![source_check];
    };

    vec![source_check, target_check]
}
