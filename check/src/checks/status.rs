use tendermint_rpc::Client;

use crate::sdk::namada::Sdk;

use super::DoCheck;

#[derive(Clone, Debug, Default)]
pub struct StatusCheck;

impl DoCheck for StatusCheck {
    async fn check(&self, sdk: &Sdk, _state: &mut crate::state::State) -> Result<(), String> {
        let client = sdk.namada.clone_client();
        let status = client
            .status()
            .await
            .map_err(|e| format!("Failed to query status: {e}"))?;

        tracing::info!(
            "Node moniker: {}, Node voting power {}, Node is catching up: {}",
            status.node_info.moniker,
            status.validator_info.power.to_string(),
            status.sync_info.catching_up
        );
        Ok(())
    }

    fn timing(&self) -> u32 {
        20
    }

    fn name(&self) -> String {
        "StatusCheck".to_string()
    }
}
