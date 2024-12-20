use namada_sdk::rpc;

use crate::sdk::namada::Sdk;

use super::DoCheck;

#[derive(Clone, Debug, Default)]
pub struct EpochCheck {}

impl DoCheck for EpochCheck {
    async fn check(sdk: &Sdk, state: &mut crate::state::State) -> Result<(), String> {
        let client = sdk.namada.clone_client();
        let last_epoch = rpc::query_epoch(&client).await;

        match last_epoch {
            Ok(epoch) => {
                let current_epoch = epoch.0;
                if state.last_epoch <= current_epoch {
                    state.last_epoch = current_epoch;
                    tracing::info!("Epoch ok");
                    Ok(())
                } else {
                    Err(format!(
                        "Epoch decreased: before: {} -> after {}",
                        state.last_epoch, epoch.0
                    ))
                }
            }
            Err(e) => Err(format!("Failed to query last epoch: {}", e)),
        }
    }

    fn timing() -> u32 {
        15
    }

    fn to_string() -> String {
        "EpochCheck".to_string()
    }
}
