use namada_sdk::rpc;

use crate::sdk::namada::Sdk;

use super::DoCheck;

#[derive(Clone, Debug, Default)]
pub struct InflationCheck;

impl DoCheck for InflationCheck {
    async fn check(&self, sdk: &Sdk, state: &mut crate::state::State) -> Result<(), String> {
        let native_token = rpc::query_native_token(&sdk.namada.client)
            .await
            .map_err(|e| e.to_string())?;
        let current_total_supply = rpc::get_token_total_supply(&sdk.namada.client, &native_token)
            .await
            .map_err(|e| format!("Failed to query total supply: {e}"))?;

        if state.last_total_supply <= current_total_supply {
            state.last_total_supply = current_total_supply;
            tracing::info!("Total supply ok");
            Ok(())
        } else {
            Err(format!(
                "Total supply decreases: before: {} -> after {}",
                state.last_total_supply, current_total_supply
            ))
        }
    }

    fn timing(&self) -> u32 {
        20
    }

    fn name(&self) -> String {
        "InflationCheck".to_string()
    }
}
