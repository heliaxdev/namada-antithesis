use namada_sdk::address::Address;
use namada_sdk::events::extend::Height as HeightAttr;
use namada_sdk::events::Event;
use namada_sdk::ibc::apps::transfer::types::ack_success_b64;
use namada_sdk::ibc::apps::transfer::types::packet::PacketData;
use namada_sdk::ibc::core::channel::types::acknowledgement::{
    Acknowledgement, AcknowledgementStatus,
};
use namada_sdk::ibc::core::channel::types::msgs::PacketMsg;
use namada_sdk::ibc::core::handler::types::msgs::MsgEnvelope;
use namada_sdk::ibc::core::host::types::identifiers::{ChannelId, PortId, Sequence};
use namada_sdk::ibc::event::IbcEventType;
use namada_sdk::ibc::event::PacketAck as PacketAckAttr;
use namada_sdk::ibc::{decode_message, IbcMessage};
use namada_sdk::queries::RPC;
use namada_sdk::tx::Tx;

use namada_sdk::io::Client;
use sha2::{Digest, Sha256};

use crate::constants::IBC_TIMEOUT_HEIGHT_OFFSET;
use crate::context::Ctx;
use crate::error::QueryError;
use crate::types::{Alias, Height};
use crate::utils::{get_block_height, wait_block_settlement, RetryConfig};

pub fn is_native_denom(denom: &str) -> bool {
    !denom.contains('/')
}

pub fn ibc_denom(channel_id: &ChannelId, base_token: &str) -> String {
    format!("transfer/{channel_id}/{base_token}")
}

pub fn base_denom(denom: &str) -> String {
    denom.split('/').skip(2).collect::<Vec<_>>().join("/")
}

pub fn ibc_token_address(denom: &str) -> Address {
    namada_sdk::ibc::trace::ibc_token(denom)
}

pub fn cosmos_denom_hash(denom: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(denom);
    let hash = hasher.finalize();
    format!("ibc/{hash:X}")
}

/// Get the IBC packet sequence.
/// This function assumes that the workload has submitted only one tx with send_packet at once.
pub async fn get_ibc_packet_sequence(
    ctx: &Ctx,
    sender: &Alias,
    receiver: &Alias,
    height: Height,
    from_namada: bool,
    retry_config: RetryConfig,
) -> Result<u64, QueryError> {
    let wallet = ctx.namada.wallet.read().await;
    let sender = if from_namada {
        wallet
            .find_address(&sender.name)
            .ok_or_else(|| QueryError::Wallet(format!("No sender address: {}", sender.name)))?
            .into_owned()
            .to_string()
    } else {
        sender.name.clone()
    };
    let receiver = if from_namada {
        receiver.name.clone()
    } else {
        wallet
            .find_address(&receiver.name)
            .ok_or_else(|| QueryError::Wallet(format!("No receiver address: {}", receiver.name)))?
            .into_owned()
            .to_string()
    };
    drop(wallet);
    let query_fn: Box<dyn Fn() -> _> = if from_namada {
        Box::new(|| ctx.namada.client.block_results(height))
    } else {
        Box::new(|| ctx.cosmos.client.block_results(height))
    };

    let block_results = tryhard::retry_fn(query_fn)
        .with_config(retry_config)
        .on_retry(|attempt, _, error| {
            let error = error.to_string();
            async move {
                tracing::info!("Retry {} due to {}...", attempt, error);
            }
        })
        .await
        .map_err(|e| {
            QueryError::Rpc(namada_sdk::error::Error::Query(
                namada_sdk::error::QueryError::General(e.to_string()),
            ))
        })?;

    get_packet_sequence(block_results, &sender, &receiver, from_namada)
}

fn get_packet_sequence(
    block_results: tendermint_rpc::endpoint::block_results::Response,
    sender: &str,
    receiver: &str,
    from_namada: bool,
) -> Result<u64, QueryError> {
    let events = if from_namada {
        block_results.end_block_events.expect("events should exist")
    } else {
        block_results
            .txs_results
            .expect("results should exist")
            .into_iter()
            .flat_map(|result| result.events)
            .collect()
    };
    for event in events {
        if event.kind == "send_packet" {
            let mut is_target = false;
            for attr in &event.attributes {
                if attr.key_str().expect("key should exist") == "packet_data" {
                    let val = attr.value_str().expect("value should exist");
                    let packet_data: PacketData =
                        serde_json::from_str(val).expect("packet should be parsable");
                    if packet_data.sender.as_ref() == sender
                        && packet_data.receiver.as_ref() == receiver
                    {
                        is_target = true;
                        break;
                    }
                }
            }
            if is_target {
                for attr in &event.attributes {
                    if attr.key_str().expect("key should exist") == "packet_sequence" {
                        return Ok(attr
                            .value_str()
                            .expect("value should exist")
                            .parse()
                            .expect("sequence should be parsable"));
                    }
                }
            }
        }
    }
    Err(QueryError::Ibc(format!(
        "Packet not found: sender {}, receiver {}",
        sender, receiver
    )))
}

pub async fn is_ibc_transfer_successful(
    ctx: &Ctx,
    src_channel_id: &ChannelId,
    dest_channel_id: &ChannelId,
    sequence: Sequence,
    retry_config: RetryConfig,
) -> Result<bool, QueryError> {
    let event = get_ibc_event(
        ctx,
        "acknowledge_packet",
        src_channel_id,
        dest_channel_id,
        sequence,
        retry_config,
    )
    .await?;

    // Retrieve the height where the tx with packet ack was executed
    let height = event
        .read_attribute::<HeightAttr>()
        .expect("Height should exist");

    // Retrieve the block at the height
    let block = tryhard::retry_fn(|| ctx.namada.client.block(height.0))
        .with_config(retry_config)
        .on_retry(|attempt, _, error| {
            let error = error.to_string();
            async move {
                tracing::info!("Retry {} due to {}...", attempt, error);
            }
        })
        .await
        .map_err(|e| {
            QueryError::Ibc(format!(
                "Querying block including tx with packet ack failed: {e}"
            ))
        })?;

    // Look for the corresponding tx and check the ack
    for tx_bytes in block.block.data() {
        let tx = Tx::try_from_bytes(tx_bytes)
            .map_err(|e| QueryError::Ibc(format!("Decoding Tx failed: {e}")))?;

        for cmts in &tx.header.batch {
            let Some(data) = tx.get_data_section(cmts.data_sechash()) else {
                continue;
            };

            let Ok(IbcMessage::Envelope(envelope)) =
                decode_message::<namada_sdk::token::Transfer>(&data)
            else {
                continue;
            };

            let MsgEnvelope::Packet(PacketMsg::Ack(msg)) = *envelope else {
                continue;
            };

            if msg.packet.seq_on_a == sequence
                && msg.packet.chan_id_on_a == *src_channel_id
                && msg.packet.chan_id_on_b == *dest_channel_id
            {
                return Ok(
                    msg.acknowledgement == AcknowledgementStatus::success(ack_success_b64()).into()
                );
            }
        }
    }

    Err(QueryError::Ibc(format!("Tx with packet ack was not found: src_channel {src_channel_id}, dest_channel {dest_channel_id}, sequence {sequence}")))
}

pub async fn is_recv_packet(
    ctx: &Ctx,
    src_channel_id: &ChannelId,
    dest_channel_id: &ChannelId,
    sequence: Sequence,
    retry_config: RetryConfig,
) -> Result<(bool, Height), QueryError> {
    let event = get_ibc_event(
        ctx,
        "write_acknowledgement",
        src_channel_id,
        dest_channel_id,
        sequence,
        retry_config,
    )
    .await?;
    let height = event
        .read_attribute::<HeightAttr>()
        .expect("Height should exist");
    let ack = event
        .read_attribute::<PacketAckAttr>()
        .expect("Ack should exist");

    let success = Acknowledgement::from(AcknowledgementStatus::success(ack_success_b64()));
    let is_successful =
        ack == std::str::from_utf8(success.as_bytes()).expect("Decoding shouldn't fail");

    Ok((is_successful, height.into()))
}

async fn get_ibc_event(
    ctx: &Ctx,
    ibc_event_type: &str,
    src_channel_id: &ChannelId,
    dest_channel_id: &ChannelId,
    sequence: Sequence,
    retry_config: RetryConfig,
) -> Result<Event, QueryError> {
    let ibc_event_type = IbcEventType(ibc_event_type.to_string());
    let port_id = PortId::transfer();
    let shell = RPC.shell();

    // Look for recv_packet event with the sequence
    let mut height = get_block_height(ctx, retry_config).await?;
    let timeout_height = height + IBC_TIMEOUT_HEIGHT_OFFSET * 2;
    while height < timeout_height {
        match shell
            .ibc_packet(
                &ctx.namada.client,
                &ibc_event_type,
                &port_id,
                src_channel_id,
                &port_id,
                dest_channel_id,
                &sequence,
            )
            .await
        {
            Ok(Some(event)) => return Ok(event),
            _ => {
                wait_block_settlement(ctx, height, retry_config).await;
                height += 1;
                tracing::info!("Retry IBC {ibc_event_type} event query at {height}...");
            }
        }
    }
    Err(QueryError::Ibc(format!(
        "Event not found: ibc_event_type {ibc_event_type}, sequence {sequence}"
    )))
}
