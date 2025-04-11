use std::collections::HashMap;

use namada_sdk::token;
use serde_json::json;
use typed_builder::TypedBuilder;

use crate::check::{CheckContext, CheckInfo};
use crate::context::Ctx;
use crate::error::CheckError;
use crate::types::{Alias, Amount, Balance, Fee};
use crate::utils::{get_shielded_balance, shielded_sync_with_retry, RetryConfig};

#[derive(TypedBuilder)]
pub struct BalanceShieldedSource {
    target: Alias,
    pre_balance: Balance,
    amount: Amount,
}

impl BalanceShieldedSource {
    pub fn target(&self) -> &Alias {
        &self.target
    }

    pub fn pre_balance(&self) -> Balance {
        self.pre_balance
    }

    pub fn amount(&self) -> Amount {
        self.amount
    }
}

impl CheckContext for BalanceShieldedSource {
    fn summary(&self) -> String {
        format!("balance-shielded/source/{}", self.target.name)
    }

    async fn do_check(
        &self,
        ctx: &Ctx,
        fees: &HashMap<Alias, Fee>,
        check_info: CheckInfo,
        retry_config: RetryConfig,
    ) -> Result<(), CheckError> {
        shielded_sync_with_retry(
            ctx,
            &self.target,
            Some(check_info.execution_height),
            true,
            retry_config,
        )
        .await?;

        let post_balance = get_shielded_balance(ctx, &self.target, retry_config)
            .await?
            .ok_or_else(|| {
                antithesis_sdk::assert_unreachable!(
                    "BalanceShieldedSource target doesn't exist.",
                    &json!({
                        "source_alias": self.target,
                        "pre_balance": self.pre_balance,
                        "amount": self.amount,
                        "execution_height": check_info.execution_height,
                        "check_height": check_info.check_height,
                    })
                );
                CheckError::State(format!(
                    "BalanceShieldedSource check error: {} balance doesn't exist",
                    self.target.name
                ))
            })?;

        let fee = fees.get(&self.target).cloned().unwrap_or_default();

        let check_balance = self
            .pre_balance
            .checked_sub(token::Amount::from_u64(self.amount + fee))
            .ok_or_else(|| {
                CheckError::State(format!(
                    "BalanceShieldedSource check error: {} balance is underflowing",
                    self.target.name
                ))
            })?;

        let details = json!({
            "source_alias": self.target,
            "pre_balance": self.pre_balance,
            "amount": self.amount,
            "paid_fee": fee,
            "post_balance": post_balance,
            "execution_height": check_info.execution_height,
            "check_height": check_info.check_height
        });

        antithesis_sdk::assert_always!(
            post_balance.eq(&check_balance),
            "BalanceShielded source decreased",
            &details
        );

        if post_balance.eq(&check_balance) {
            Ok(())
        } else {
            tracing::error!("{}", details);
            Err(CheckError::State(format!("BalanceShieldedSource check error: post source amount is not equal to pre balance - amount - fee: {} - {} - {fee} = {check_balance} != {post_balance}", self.pre_balance, self.amount)))
        }
    }
}
