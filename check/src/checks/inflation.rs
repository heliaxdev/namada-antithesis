use namada_sdk::rpc;

use crate::sdk::namada::Sdk;

use super::DoCheck;

#[derive(Clone, Debug, Default)]
pub struct InflationCheck;

impl DoCheck for InflationCheck {
    async fn check(&self, sdk: &Sdk, state: &mut crate::state::State) -> Result<(), String> {
        let client = sdk.namada.clone_client();
        let native_token = match rpc::query_native_token(&client).await {
            Ok(address) => address,
            Err(e) => {
                return Err(e.to_string());
            }
        };

        let total_supply = rpc::get_token_total_supply(&client, &native_token).await;

        match total_supply {
            Ok(current_total_supply) => {
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
            Err(e) => Err(format!("Failed to query total supply: {}", e)),
        }
    }

    fn timing(&self) -> u32 {
        20
    }

    fn name(&self) -> String {
        "InflationCheck".to_string()
    }
}
