use std::collections::HashMap;

use namada_sdk::ibc::core::host::types::identifiers::ChannelId;
use serde_json::json;
use typed_builder::TypedBuilder;

use crate::check::{CheckContext, CheckInfo};
use crate::context::Ctx;
use crate::error::CheckError;
use crate::types::{Alias, Fee};
use crate::utils::{get_ibc_packet_sequence, is_ibc_transfer_successful, RetryConfig};

#[derive(TypedBuilder)]
pub struct AckIbcTransfer {
    source: Alias,
    receiver: Alias,
    src_channel_id: ChannelId,
    dest_channel_id: ChannelId,
}

impl CheckContext for AckIbcTransfer {
    fn summary(&self) -> String {
        format!(
            "ack-ibc-transfer/{}/{}",
            self.source.name, self.receiver.name
        )
    }

    async fn do_check(
        &self,
        ctx: &Ctx,
        _fees: &HashMap<Alias, Fee>,
        check_info: CheckInfo,
        retry_config: RetryConfig,
    ) -> Result<(), CheckError> {
        let sequence = get_ibc_packet_sequence(
            ctx,
            &self.source,
            &self.receiver,
            check_info.execution_height,
            retry_config,
        )
        .await?;
        let is_successful = is_ibc_transfer_successful(
            ctx,
            &self.src_channel_id,
            &self.dest_channel_id,
            sequence.into(),
            retry_config,
        )
        .await?;

        let details = json!({
            "source_alias": self.source,
            "receiver": self.receiver,
            "src_channel_id": self.src_channel_id,
            "dest_channel_id": self.dest_channel_id,
            "sequence": sequence,
            "execution_height": check_info.execution_height,
            "check_height": check_info.check_height,
        });

        antithesis_sdk::assert_always!(is_successful, "IBC transfer was acknowledged", &details);

        Ok(())
    }
}
