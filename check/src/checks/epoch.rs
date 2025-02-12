use namada_sdk::rpc;

use crate::sdk::namada::Sdk;

use super::DoCheck;

#[derive(Clone, Debug, Default)]
pub struct EpochCheck;

impl DoCheck for EpochCheck {
    async fn check(&self, sdk: &Sdk, state: &mut crate::state::State) -> Result<(), String> {
        let current_epoch = rpc::query_epoch(&sdk.namada.client)
            .await
            .map_err(|e| format!("Failed to query last epoch: {e}"))?
            .into();

        if state.last_epoch <= current_epoch {
            state.last_epoch = current_epoch;
            tracing::info!("Epoch ok");
            Ok(())
        } else {
            Err(format!(
                "Epoch decreased: before: {} -> after {}",
                state.last_epoch, current_epoch
            ))
        }
    }

    fn timing(&self) -> u32 {
        15
    }

    fn name(&self) -> String {
        "EpochCheck".to_string()
    }
}
