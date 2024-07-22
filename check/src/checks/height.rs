use namada_sdk::{queries::Client, Namada};

use crate::sdk::namada::Sdk;

use super::DoCheck;

#[derive(Clone, Debug, Default)]
pub struct HeightCheck {}

impl DoCheck for HeightCheck {
    async fn do_check(sdk: &Sdk, state: &mut crate::state::State) -> Result<(), String> {
        let client = sdk.namada.client();
        let last_block = client.latest_block().await;
        match last_block {
            Ok(block) => {
                let current_block_height = u64::from(block.block.header.height);
                if state.last_block_height <= current_block_height {
                    state.last_block_height = current_block_height;
                    tracing::info!("Block height before: {}, after {}", state.last_block_height, current_block_height);
                    Ok(())
                } else {
                    Err("Block height didnt increase".to_string())
                }
            }
            Err(e) => Err(format!("Failed to query last block: {}", e)),
        }
    }

    fn to_string() -> String {
        "HeightCheck".to_string()
    }
}
