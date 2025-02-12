use namada_sdk::rpc;

use crate::sdk::namada::Sdk;

use super::DoCheck;

#[derive(Clone, Debug, Default)]
pub struct HeightCheck;

impl DoCheck for HeightCheck {
    async fn check(&self, sdk: &Sdk, state: &mut crate::state::State) -> Result<(), String> {
        let last_block = rpc::query_block(&sdk.namada.client)
            .await
            .map_err(|e| format!("Failed to query last block: {e}"))?
            .ok_or("No block found".to_string())?;

        let current_block_height = last_block.height.into();
        if state.last_block_height <= current_block_height {
            tracing::info!(
                "Block height ok ({} -> {})",
                state.last_block_height,
                current_block_height
            );
            state.last_block_height = current_block_height;
            Ok(())
        } else {
            Err(format!(
                "Block height didnt increase: before: {} -> after {}",
                state.last_block_height, current_block_height
            ))
        }
    }

    fn timing(&self) -> u32 {
        6
    }

    fn name(&self) -> String {
        "HeightCheck".to_string()
    }
}
