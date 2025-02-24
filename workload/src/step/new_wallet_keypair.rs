use serde_json::json;

use crate::assert_step;
use crate::code::Code;
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task};

use super::utils;

#[derive(Clone, Debug, Default)]
pub struct NewWalletKeyPair;

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

    fn assert(&self, code: &Code) {
        let is_fatal = code.is_fatal();
        let is_failed = code.is_failed();
        let is_skipped = code.is_skipped();
        let is_successful = code.is_successful();

        let details = json!({"outcome": code.code()});

        if is_fatal {
            assert_step!("Fatal NewWalletKeyPair", details)
        } else if is_failed {
            assert_step!("Failed NewWalletKeyPair", details)
        } else if is_skipped {
            assert_step!("Skipped NewWalletKeyPair", details)
        } else if is_successful {
            assert_step!("Done NewWalletKeyPair", details)
        } else {
            assert_step!("Unknown Code NewWalletKeyPair ", details)
        }
    }
}
