use tendermint_rpc::Client;

use crate::sdk::namada::Sdk;

use super::DoCheck;

#[derive(Clone, Debug, Default)]
pub struct HeightCheck;

impl DoCheck for HeightCheck {
    async fn check(&self, sdk: &Sdk, state: &mut crate::state::State) -> Result<(), String> {
        let client = sdk.namada.clone_client();
        let last_block = client.latest_block().await;

        match last_block {
            Ok(block) => {
                let current_block_height = u64::from(block.block.header.height);
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
            Err(e) => Err(format!("Failed to query last block: {}", e)),
        }
    }

    fn timing(&self) -> u32 {
        6
    }

    fn name(&self) -> String {
        "HeightCheck".to_string()
    }
}
