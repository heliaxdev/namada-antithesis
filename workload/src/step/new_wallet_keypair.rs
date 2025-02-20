use async_trait::async_trait;

use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task};

use super::utils;

#[derive(Clone, Debug, Default)]
pub struct NewWalletKeyPair;

#[async_trait]
impl StepContext for NewWalletKeyPair {
    fn name(&self) -> String {
        "new-walleet-keypair".to_string()
    }

    async fn is_valid(&self, _sdk: &Sdk, _state: &State) -> Result<bool, StepError> {
        Ok(true)
    }

    async fn build_task(&self, _sdk: &Sdk, state: &mut State) -> Result<Vec<Task>, StepError> {
        let alias = utils::random_alias(state);
        Ok(vec![Task::NewWalletKeyPair(
            task::new_wallet_keypair::NewWalletKeyPair::builder()
                .source(alias)
                .build(),
        )])
    }
}
