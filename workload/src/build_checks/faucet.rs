use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};

use crate::{check::Check, entities::Alias, sdk::namada::Sdk, state::State};

pub async fn faucet_build_check(
    sdk: &Sdk,
    target: Alias,
    amount: u64,
    retry_config: RetryFutureConfig<ExponentialBackoff, NoOnRetry>,
    state: &State,
) -> Vec<Check> {
    let check = if let Some(pre_balance) =
        super::utils::get_balance(sdk, target.clone(), retry_config).await
    {
        Check::BalanceTarget(target, pre_balance, amount, state.clone())
    } else {
        tracing::info!("retrying ...");
        return vec![];
    };

    vec![check]
}
