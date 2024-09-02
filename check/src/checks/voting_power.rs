use namada_sdk::Namada;
use tendermint_rpc::Client;

use crate::sdk::namada::Sdk;

use super::DoCheck;

#[derive(Clone, Debug, Default)]
pub struct VotingPowerCheck {}

impl DoCheck for VotingPowerCheck {
    async fn check(sdk: &Sdk, state: &mut crate::state::State) -> Result<(), String> {
        let client = sdk.namada.client();
        let status = client.status().await;

        match status {
            Ok(status) => {
                let block_height = status.sync_info.latest_block_height;
                let validators = client
                    .validators(block_height, tendermint_rpc::Paging::All)
                    .await;
                match validators {
                    Ok(validators) => {
                        let mut total_vp = 0;
                        for validator in validators.validators.clone() {
                            total_vp += validator.power();
                        }
                        tracing::info!("Total vp: {}", total_vp);
                        for validator in validators.validators {
                            let vp = validator.power();
                            let percentage_vp = (vp as f32) / (total_vp as f32);
                            tracing::info!("Validator: {}, voting power: {}, percentage: {}%", validator.address, vp, percentage_vp);
                        }
                    }
                    Err(e) => Err(format!("Failed to query validators: {}", e))?,
                }
                Ok(())
            }
            Err(e) => Err(format!("Failed to query status: {}", e)),
        }
    }

    fn timing() -> u32 {
        30
    }

    fn to_string() -> String {
        "VotingPowerCheck".to_string()
    }
}
