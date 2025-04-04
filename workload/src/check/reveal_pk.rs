use std::collections::HashMap;

use serde_json::json;
use typed_builder::TypedBuilder;

use crate::check::{CheckContext, CheckInfo};
use crate::context::Ctx;
use crate::error::CheckError;
use crate::types::{Alias, Fee};
use crate::utils::{is_pk_revealed, RetryConfig};

#[derive(TypedBuilder)]
pub struct RevealPk {
    target: Alias,
}

impl CheckContext for RevealPk {
    fn summary(&self) -> String {
        format!("reveal/{}", self.target.name)
    }

    async fn do_check(
        &self,
        ctx: &Ctx,
        _fees: &HashMap<Alias, Fee>,
        check_info: CheckInfo,
        retry_config: RetryConfig,
    ) -> Result<(), CheckError> {
        let was_pk_revealed = is_pk_revealed(ctx, &self.target, retry_config).await?;

        antithesis_sdk::assert_always!(
            was_pk_revealed,
            "The public key was revealed correctly",
            &json!({
                "target": self.target.name,
                "execution_height": check_info.execution_height,
                "check_height": check_info.check_height,
            })
        );
        if was_pk_revealed {
            Ok(())
        } else {
            Err(CheckError::State(format!(
                "RevealPk check error: pk for {} was not revealed",
                self.target.name
            )))
        }
    }
}
