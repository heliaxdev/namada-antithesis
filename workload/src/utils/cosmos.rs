use std::str::FromStr;

use cosmrs::proto::prost::{Message, Name};
use cosmrs::tx::{AuthInfo, Body, Fee, SignDoc, SignerInfo};
use cosmrs::Any;
use ibc_proto::cosmos::auth::v1beta1::query_client::QueryClient;
use ibc_proto::cosmos::auth::v1beta1::{BaseAccount, QueryAccountRequest};
use ibc_proto::cosmos::base::v1beta1::Coin;
use ibc_proto::ibc::apps::transfer::v1::MsgTransfer;
use namada_sdk::ibc::core::host::types::identifiers::{ChannelId, PortId};
use tendermint_rpc::Client;

use crate::constants::{COSMOS_CHAIN_ID, COSMOS_FEE_AMOUNT, COSMOS_FEE_TOKEN, COSMOS_GAS_LIMIT};
use crate::context::Ctx;
use crate::error::{QueryError, TaskError};
use crate::types::{Amount, Height};

pub fn build_cosmos_ibc_transfer(
    sender: &str,
    receiver: &str,
    denom: &str,
    amount: Amount,
    src_port_id: &PortId,
    src_channel_id: &ChannelId,
    memo: Option<&str>,
) -> Any {
    let token = Coin {
        denom: denom.to_string(),
        amount: amount.to_string(),
    };

    let msg = MsgTransfer {
        source_port: src_port_id.to_string(),
        source_channel: src_channel_id.to_string(),
        token: Some(token),
        sender: sender.to_string(),
        receiver: receiver.to_string(),
        timeout_height: None,
        timeout_timestamp: 0,
        memo: memo.unwrap_or_default().to_string(),
    };

    Any {
        type_url: MsgTransfer::type_url(),
        value: msg.encode_to_vec(),
    }
}

pub async fn execute_cosmos_tx(ctx: &Ctx, any_msg: Any) -> Result<Height, TaskError> {
    let body = Body::new(vec![any_msg], "", 0u32);
    let signing_key = &ctx.cosmos.signing_key;

    // Account
    let mut grpc_client = QueryClient::connect(ctx.cosmos.grpc_endpoint.clone())
        .await
        .expect("invalid gRPC");
    let res = grpc_client
        .account(QueryAccountRequest {
            address: ctx.cosmos.account.to_string(),
        })
        .await
        .map_err(|e| QueryError::Grpc(e.to_string()))?;
    let any = res.into_inner().account.expect("Account should exist");
    let base_account: BaseAccount = prost::Message::decode(any.value.as_slice())
        .map_err(|e| QueryError::Convert(e.to_string()))?;

    // AuthInfo
    let public_key = signing_key.public_key();
    let signer_info = SignerInfo::single_direct(Some(public_key), base_account.sequence);
    let fee = Fee::from_amount_and_gas(
        cosmrs::Coin {
            denom: COSMOS_FEE_TOKEN.parse().expect("token should be parsable"),
            amount: COSMOS_FEE_AMOUNT.into(),
        },
        COSMOS_GAS_LIMIT,
    );

    let auth_info = AuthInfo {
        signer_infos: vec![signer_info],
        fee,
    };

    let sign_doc = SignDoc::new(
        &body,
        &auth_info,
        &tendermint::chain::Id::from_str(COSMOS_CHAIN_ID).expect("chain ID should be parsable"),
        base_account.account_number,
    )
    .map_err(|e| TaskError::CosmosTx(e.to_string()))?;
    let tx_raw = sign_doc
        .sign(signing_key)
        .map_err(|e| TaskError::CosmosTx(e.to_string()))?;

    let tx_bytes = tx_raw.to_bytes().expect("tx should be encoded");

    let response = ctx
        .cosmos
        .client
        .broadcast_tx_commit(tx_bytes)
        .await
        .map_err(|e| TaskError::CosmosTx(e.to_string()))?;

    if response.check_tx.code.is_ok() && response.tx_result.code.is_ok() {
        Ok(response.height.into())
    } else if response.check_tx.code.is_err() {
        Err(TaskError::CosmosTx(response.check_tx.log))
    } else {
        Err(TaskError::CosmosTx(response.tx_result.log))
    }
}
