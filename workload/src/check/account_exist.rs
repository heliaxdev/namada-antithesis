use std::collections::{BTreeSet, HashMap};

use serde_json::json;
use typed_builder::TypedBuilder;

use crate::check::{CheckContext, CheckInfo};
use crate::context::Ctx;
use crate::error::CheckError;
use crate::types::{Alias, Fee, Threshold};
use crate::utils::{get_account_info, RetryConfig};

#[derive(TypedBuilder)]
pub struct AccountExist {
    target: Alias,
    threshold: Threshold,
    sources: BTreeSet<Alias>,
}

impl CheckContext for AccountExist {
    fn summary(&self) -> String {
        format!("account-exist/{}", self.target.name)
    }

    async fn do_check(
        &self,
        ctx: &Ctx,
        _fees: &HashMap<Alias, Fee>,
        check_info: CheckInfo,
        retry_config: RetryConfig,
    ) -> Result<(), CheckError> {
        let (target_address, account) = get_account_info(ctx, &self.target, retry_config).await?;
        let account = account.ok_or_else(|| {
            antithesis_sdk::assert_unreachable!(
                "OnChain account doesn't exist",
                &json!({
                    "target_alias": self.target,
                    "target": target_address.to_pretty_string(),
                    "account": "",
                    "threshold": self.threshold,
                    "sources": self.sources,
                    "execution_height": check_info.execution_height,
                    "check_height": check_info.check_height
                })
            );
            CheckError::State(format!(
                "AccountExist check error: account {} doesn't exist",
                self.target.name
            ))
        })?;

        let is_threshold_ok = account.threshold == self.threshold as u8;
        let is_sources_ok = self.sources.len() == account.public_keys_map.idx_to_pk.len();
        let is_valid = is_sources_ok && is_threshold_ok;

        let details = json!({
            "target_alias": self.target,
            "target": target_address.to_pretty_string(),
            "account": account,
            "threshold": self.threshold,
            "sources": self.sources,
            "execution_height": check_info.execution_height,
            "check_height": check_info.check_height
        });

        antithesis_sdk::assert_always!(is_valid, "OnChain account is valid", &details);

        if is_valid {
            Ok(())
        } else {
            tracing::error!("{}", details);
            Err(CheckError::State(format!(
                "AccountExist check error: account {} is invalid",
                self.target.name
            )))
        }
    }
}
