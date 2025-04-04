use std::collections::HashMap;

use serde_json::json;
use typed_builder::TypedBuilder;

use crate::check::{CheckContext, CheckInfo};
use crate::context::Ctx;
use crate::error::CheckError;
use crate::types::{Alias, Fee};
use crate::utils::{is_validator, RetryConfig};

#[derive(TypedBuilder)]
pub struct ValidatorAccount {
    target: Alias,
}

impl CheckContext for ValidatorAccount {
    fn summary(&self) -> String {
        format!("is-validator/{}", self.target.name)
    }

    async fn do_check(
        &self,
        ctx: &Ctx,
        _fees: &HashMap<Alias, Fee>,
        check_info: CheckInfo,
        retry_config: RetryConfig,
    ) -> Result<(), CheckError> {
        let (target_address, is_validator) = is_validator(ctx, &self.target, retry_config).await?;
        antithesis_sdk::assert_always!(
            is_validator,
            "OnChain account is a validator",
            &json!({
                "target_alias": self.target,
                "target": target_address.to_pretty_string(),
                "execution_height": check_info.execution_height,
                "check_height": check_info.check_height
            })
        );

        if is_validator {
            Ok(())
        } else {
            Err(CheckError::State(format!(
                "ValidatorAccount check error: post target {} state isn't a validator",
                self.target.name
            )))
        }
    }
}
