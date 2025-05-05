use std::collections::HashMap;

use namada_sdk::ibc::core::host::types::identifiers::ChannelId;
use serde_json::json;
use typed_builder::TypedBuilder;

use crate::check::{CheckContext, CheckInfo};
use crate::context::Ctx;
use crate::error::CheckError;
use crate::types::{Alias, Fee};

use crate::utils::{get_ibc_packet_sequence, is_recv_packet, RetryConfig};

#[derive(TypedBuilder)]
pub struct RecvIbcPacket {
    sender: Alias,
    target: Alias,
    src_channel_id: ChannelId,
    dest_channel_id: ChannelId,
}

impl CheckContext for RecvIbcPacket {
    fn summary(&self) -> String {
        format!("recv-ibc-packet/{}/{}", self.sender.name, self.target.name)
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
            &self.sender,
            &self.target,
            check_info.execution_height,
            false,
            retry_config,
        )
        .await?;
        let (is_successful, _) = is_recv_packet(
            ctx,
            &self.src_channel_id,
            &self.dest_channel_id,
            sequence.into(),
            retry_config,
        )
        .await?;

        let details = json!({
            "sender": self.sender,
            "target_alias": self.target,
            "src_channel_id": self.src_channel_id,
            "dest_channel_id": self.dest_channel_id,
            "sequence": sequence,
            "execution_height": check_info.execution_height,
            "check_height": check_info.check_height,
        });

        antithesis_sdk::assert_always!(is_successful, "IBC packet was received", &details);

        if is_successful {
            Ok(())
        } else {
            tracing::error!("{}", details);
            Err(CheckError::State(
                "IBC packet was not received properly".to_string(),
            ))
        }
    }
}
