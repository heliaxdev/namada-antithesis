use serde::{Deserialize, Serialize};

use crate::sdk::namada::Sdk;

use super::DoCheck;

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct LatestHeightResponse {
    pub block_height: u64,
}

#[derive(Clone, Debug, Default)]
pub struct MaspIndexerHeightCheck;

impl DoCheck for MaspIndexerHeightCheck {
    async fn check(&self, sdk: &Sdk, state: &mut crate::state::State) -> Result<(), String> {
        let url = format!("{}/api/v1/height", sdk.masp_indexer_url);
        let response = reqwest::get(&url)
            .await
            .map_err(|e| format!("Error while requesting height from masp indexer: {e}"))?;

        if response.status() != reqwest::StatusCode::OK {
            return Err(format!(
                "Error while requesting height from masp indexer: status code was {}",
                response.status()
            ));
        }

        let parsed = response
            .json::<LatestHeightResponse>()
            .await
            .map_err(|e| format!("Error while parsing height from masp indexer: {e}"))?;

        let current_block_height = parsed.block_height;
        if state.last_block_height_masp_indexer <= current_block_height {
            tracing::info!(
                "Masp indexer block height ok ({} -> {})",
                state.last_block_height_masp_indexer,
                current_block_height
            );
            state.last_block_height_masp_indexer = current_block_height;
            Ok(())
        } else {
            Err(format!(
                "Masp indexer height didn't increase: before: {} -> after {}",
                state.last_block_height_masp_indexer, current_block_height
            ))
        }
    }

    fn timing(&self) -> u32 {
        12
    }

    fn name(&self) -> String {
        "MaspIndexerHeightCheck".to_string()
    }
}
