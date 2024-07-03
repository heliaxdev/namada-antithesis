use namada_sdk::{rpc, Namada};

use crate::sdk::namada::Sdk;

use super::DoCheck;

#[derive(Clone, Debug, Default)]
pub struct EpochCheck {}

impl DoCheck for EpochCheck {
    async fn do_check(sdk: &Sdk, state: &mut crate::state::State) -> Result<(), String> {
        let client = sdk.namada.client();
        let last_epoch = rpc::query_epoch(client).await;
        match last_epoch {
            Ok(epoch) => {
                let current_epoch = epoch.0;
                if state.last_epoch <= current_epoch {
                    state.last_epoch = current_epoch;
                    Ok(())
                } else {
                    Err("Epoch decreased".to_string())
                }
            }
            Err(e) => Err(format!("Failed to query last epoch: {}", e)),
        }
    }

    fn to_string() -> String {
        "EpochCheck".to_string()
    }
}
