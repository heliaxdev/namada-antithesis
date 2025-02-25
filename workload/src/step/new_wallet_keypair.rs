use serde_json::json;

use crate::code::Code;
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task};
use crate::{assert_always_step, assert_sometimes_step, assert_unrechable_step};

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

    async fn build_task(&self, _sdk: &Sdk, _state: &State) -> Result<Vec<Task>, StepError> {
        let alias = utils::random_alias();
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
            assert_unrechable_step!("Fatal NewWalletKeyPair", details)
        } else if is_failed {
            assert_unrechable_step!("Failed NewWalletKeyPair", details)
        } else if is_skipped {
            assert_sometimes_step!("Skipped NewWalletKeyPair", details)
        } else if is_successful {
            assert_always_step!("Done NewWalletKeyPair", details)
        } else {
            assert_sometimes_step!("Unknown Code NewWalletKeyPair ", details)
        }
    }
}
