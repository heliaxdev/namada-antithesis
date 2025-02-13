use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};

use crate::{
    check::Check, constants::PROPOSAL_DEPOSIT, entities::Alias, sdk::namada::Sdk, state::State,
};

pub async fn proposal(
    sdk: &Sdk,
    source: Alias,
    retry_config: RetryFutureConfig<ExponentialBackoff, NoOnRetry>,
    state: &State,
) -> Vec<Check> {
    let source_check = if let Some(pre_balance) =
        super::utils::get_balance(sdk, source.clone(), retry_config).await
    {
        Check::BalanceSource(source, pre_balance, PROPOSAL_DEPOSIT, state.clone())
    } else {
        return vec![];
    };

    vec![source_check]
}
