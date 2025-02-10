use serde::{Deserialize, Serialize};

use crate::sdk::namada::Sdk;

use super::DoCheck;

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct LatestHeightResponse {
    pub block_height: u64,
}

#[derive(Clone, Debug, Default)]
pub struct MaspIndexerHeightCheck {}

impl DoCheck for MaspIndexerHeightCheck {
    async fn check(sdk: &Sdk, state: &mut crate::state::State) -> Result<(), String> {
        let url = format!("{}/api/v1/height", sdk.masp_indexer_url);
        let masp_indexer_block_height = reqwest::get(&url).await;

        match masp_indexer_block_height {
            Ok(response) => match response.status() {
                reqwest::StatusCode::OK => match response.json::<LatestHeightResponse>().await {
                    Ok(parsed) => {
                        let current_block_height = u64::from(parsed.block_height);
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
                                "Masp indexer height didnt increase: before: {} -> after {}",
                                state.last_block_height_masp_indexer, current_block_height
                            ))
                        }
                    }
                    Err(e) => Err(format!(
                        "Error while requesting height from masp indexer: {}",
                        e
                    )),
                },
                _ => Err(format!(
                    "Error while requesting height from masp indexer: status code was {}",
                    response.status()
                )),
            },
            Err(e) => Err(format!(
                "Error while requesting height from masp indexer: {}",
                e.to_string()
            )),
        }
    }

    fn timing() -> u32 {
        12
    }

    fn to_string() -> String {
        "MaspIndexerHeightCheck".to_string()
    }
}
