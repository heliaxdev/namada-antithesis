use std::str::FromStr;

use namada_sdk::address::Address;
use namada_sdk::args::{self, TxBuilder};
use namada_sdk::signing::SigningTxData;
use namada_sdk::tx::data::GasLimit;
use namada_sdk::tx::Tx;
use namada_sdk::Namada;
use typed_builder::TypedBuilder;

use crate::check::{self, Check};
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::task::{TaskContext, TaskSettings};
use crate::types::{Alias, Amount, ValidatorAddress};
use crate::utils::{get_balance, RetryConfig};

#[derive(Clone, Debug, TypedBuilder)]
pub struct ClaimRewards {
    source: Alias,
    from_validator: ValidatorAddress,
    amount: Amount,
    settings: TaskSettings,
}

impl ClaimRewards {
    pub fn source(&self) -> &Alias {
        &self.source
    }
}

impl TaskContext for ClaimRewards {
    fn name(&self) -> String {
        "claim-rewards".to_string()
    }

    fn summary(&self) -> String {
        format!("claim-rewards/{}", self.source.name)
    }

    fn task_settings(&self) -> Option<&TaskSettings> {
        Some(&self.settings)
    }

    async fn build_tx(&self, sdk: &Sdk) -> Result<(Tx, Vec<SigningTxData>, args::Tx), StepError> {
        let wallet = sdk.namada.wallet.read().await;

        let source_address = wallet
            .find_address(&self.source.name)
            .ok_or_else(|| StepError::Wallet(format!("No source address: {}", self.source.name)))?;
        let from_validator =
            Address::from_str(&self.from_validator).expect("ValidatorAddress should be converted");

        let mut claim_rewards_tx_builder = sdk.namada.new_claim_rewards(from_validator);
        claim_rewards_tx_builder.source = Some(source_address.into_owned());
        claim_rewards_tx_builder =
            claim_rewards_tx_builder.gas_limit(GasLimit::from(self.settings.gas_limit));
        let mut signing_keys = vec![];
        for signer in &self.settings.signers {
            let public_key = wallet
                .find_public_key(&signer.name)
                .map_err(|e| StepError::Wallet(e.to_string()))?;
            signing_keys.push(public_key)
        }
        claim_rewards_tx_builder = claim_rewards_tx_builder.signing_keys(signing_keys);
        drop(wallet);

        let (claim_tx, signing_data) = claim_rewards_tx_builder
            .build(&sdk.namada)
            .await
            .map_err(|e| StepError::BuildTx(e.to_string()))?;

        Ok((claim_tx, vec![signing_data], claim_rewards_tx_builder.tx))
    }

    async fn build_checks(
        &self,
        sdk: &Sdk,
        retry_config: RetryConfig,
    ) -> Result<Vec<Check>, StepError> {
        let (_, pre_balance) = get_balance(sdk, &self.source, retry_config).await?;

        Ok(vec![Check::BalanceTarget(
            check::balance_target::BalanceTarget::builder()
                .target(self.source.clone())
                .pre_balance(pre_balance)
                .amount(self.amount)
                .allow_greater(true)
                .build(),
        )])
    }

    fn update_state(&self, state: &mut State) {
        state.increase_balance(&self.source, self.amount);
    }
}
