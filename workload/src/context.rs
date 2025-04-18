use namada_sdk::ibc::core::host::types::identifiers::ChannelId;

use crate::config::AppConfig;

mod cosmos;
mod namada;

pub struct Ctx {
    pub namada: namada::NamadaCtx,
    pub cosmos: cosmos::CosmosCtx,
    pub namada_channel_id: ChannelId,
    pub cosmos_channel_id: ChannelId,
    pub masp_indexer_url: String,
}

impl Ctx {
    pub async fn new(config: &AppConfig) -> Result<Self, String> {
        Ok(Self {
            namada: namada::namada_ctx(config).await?,
            cosmos: cosmos::CosmosCtx::new(config)?,
            // TODO: set channels
            namada_channel_id: config.namada_channel_id.parse().unwrap(),
            cosmos_channel_id: config.cosmos_channel_id.parse().unwrap(),
            masp_indexer_url: format!("{}/api/v1", config.masp_indexer_url.clone()),
        })
    }
}
