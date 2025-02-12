use tendermint_rpc::Client;

use crate::sdk::namada::Sdk;

use super::DoCheck;

#[derive(Clone, Debug, Default)]
pub struct VotingPowerCheck;

impl DoCheck for VotingPowerCheck {
    async fn check(&self, sdk: &Sdk, state: &mut crate::state::State) -> Result<(), String> {
        let client = &sdk.namada.client;
        let status = client
            .status()
            .await
            .map_err(|e| format!("Failed to query status: {e}"))?;

        let block_height = status.sync_info.latest_block_height;
        let validators = client
            .validators(block_height, tendermint_rpc::Paging::All)
            .await
            .map_err(|e| format!("Failed to query validators: {e}"))?;

        let mut total_vp = 0;
        let mut max_validator_vp = 0;
        for validator in &validators.validators {
            total_vp += validator.power();
            if max_validator_vp < validator.power() {
                max_validator_vp = validator.power();
            }
        }

        let two_third = (total_vp * 2) / 3;
        let can_halt = validators
            .validators
            .iter()
            .filter(|validator| validator.power() != max_validator_vp)
            .any(|validator| validator.power() + max_validator_vp < two_third);

        state.two_nodes_have_two_third = !can_halt;

        tracing::info!("Total vp: {}", total_vp);
        tracing::info!("Can halt: {}", can_halt);
        for validator in validators.validators {
            let vp = validator.power();
            let percentage_vp = (vp as f32) / (total_vp as f32);
            tracing::info!(
                "Validator: {}, voting power: {}, percentage: {}%",
                validator.address,
                vp,
                percentage_vp
            );
        }
        Ok(())
    }

    fn timing(&self) -> u32 {
        20
    }

    fn name(&self) -> String {
        "VotingPowerCheck".to_string()
    }
}
