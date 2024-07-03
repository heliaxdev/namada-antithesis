use namada_sdk::{rpc, Namada};

use crate::sdk::namada::Sdk;

use super::DoCheck;

#[derive(Clone, Debug, Default)]
pub struct InflationCheck {}

impl DoCheck for InflationCheck {
    async fn do_check(sdk: &Sdk, state: &mut crate::state::State) -> Result<(), String> {
        let client = sdk.namada.client();
        let native_token = rpc::query_native_token(client).await.unwrap();
        let total_supply = rpc::get_token_total_supply(client, &native_token).await;

        match total_supply {
            Ok(current_total_supply) => {
                if state.last_total_supply <= current_total_supply {
                    state.last_total_supply = current_total_supply;
                    Ok(())
                } else {
                    Err("Total supply didn't increase".to_string())
                }
            }
            Err(e) => Err(format!("Failed to query total supply: {}", e)),
        }
    }

    fn to_string() -> String {
        "InflationCheck".to_string()
    }
}
