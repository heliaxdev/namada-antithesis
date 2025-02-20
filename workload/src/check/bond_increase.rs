use async_trait::async_trait;
use namada_sdk::token;
use serde_json::json;
use typed_builder::TypedBuilder;

use crate::check::{CheckContext, CheckInfo};
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::types::{Alias, Amount, Balance, Epoch, ValidatorAddress};
use crate::utils::{get_bond, get_epoch, RetryConfig};

#[derive(TypedBuilder)]
pub struct BondIncrease {
    target: Alias,
    validator: ValidatorAddress,
    pre_bond: Balance,
    epoch: Epoch,
    amount: Amount,
}

impl BondIncrease {
    pub fn target(&self) -> &Alias {
        &self.target
    }

    pub fn validator(&self) -> &ValidatorAddress {
        &self.validator
    }

    pub fn amount(&self) -> Amount {
        self.amount
    }

    pub fn epoch(&self) -> Epoch {
        self.epoch
    }
}

#[async_trait]
impl CheckContext for BondIncrease {
    fn summary(&self) -> String {
        format!("bond/{}/{}/increase", &self.target.name, self.validator)
    }

    async fn do_check(
        &self,
        sdk: &Sdk,
        check_info: CheckInfo,
        retry_config: RetryConfig,
    ) -> Result<(), StepError> {
        let epoch = get_epoch(sdk, retry_config).await?;
        let post_bond =
            get_bond(sdk, &self.target, &self.validator, epoch + 2, retry_config).await?;
        let check_bond = self
            .pre_bond
            .checked_add(token::Amount::from_u64(self.amount))
            .ok_or_else(|| {
                StepError::StateCheck(format!(
                    "BondIncrease check error: {} bond is overflowing",
                    self.target.name
                ))
            })?;

        let details = json!({
            "target_alias": self.target,
            "validator": self.validator,
            "pre_bond": self.pre_bond,
            "amount": self.amount,
            "post_bond": post_bond,
            "execution_height": check_info.execution_height,
            "check_height": check_info.check_height,
        });

        antithesis_sdk::assert_always!(post_bond.eq(&check_bond), "Bond increased.", &details);

        if post_bond.eq(&check_bond) {
            Ok(())
        } else {
            tracing::error!("{}", details);
            Err(StepError::StateCheck(format!("BondIncrease check error: post bond amount is not equal to pre bond + amount: {} + {} = {check_bond} != {post_bond}", self.pre_bond, self.amount)))
        }
    }
}
