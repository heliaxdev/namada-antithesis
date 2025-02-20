use std::collections::BTreeSet;

use namada_sdk::{
    args::{self, TxBuilder},
    signing::SigningTxData,
    tx::{data::GasLimit, Tx},
    Namada,
};
use typed_builder::TypedBuilder;

use crate::check::Check;
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::task::{TaskContext, TaskSettings};
use crate::types::{Alias, Threshold};
use crate::utils::RetryConfig;

#[derive(Clone, TypedBuilder)]
pub struct InitAccount {
    target: Alias,
    sources: BTreeSet<Alias>,
    threshold: Threshold,
    settings: TaskSettings,
}

impl TaskContext for InitAccount {
    fn name(&self) -> String {
        "init-account".to_string()
    }

    fn summary(&self) -> String {
        format!("init-account/{}/{}", self.target.name, self.threshold)
    }

    fn task_settings(&self) -> Option<&TaskSettings> {
        Some(&self.settings)
    }

    async fn build_tx(&self, sdk: &Sdk) -> Result<(Tx, Vec<SigningTxData>, args::Tx), StepError> {
        let wallet = sdk.namada.wallet.read().await;

        let mut public_keys = vec![];
        for source in self.sources {
            let source_pk = wallet
                .find_public_key(&source.name)
                .map_err(|e| StepError::Wallet(e.to_string()))?;
            public_keys.push(source_pk);
        }

        let fee_payer = wallet
            .find_public_key(&self.settings.gas_payer.name)
            .map_err(|e| StepError::Wallet(e.to_string()))?;

        let mut init_account_builder = sdk
            .namada
            .new_init_account(public_keys, Some(self.threshold as u8))
            .initialized_account_alias(self.target.name.clone())
            .wallet_alias_force(true);

        init_account_builder =
            init_account_builder.gas_limit(GasLimit::from(self.settings.gas_limit));
        init_account_builder = init_account_builder.wrapper_fee_payer(fee_payer);

        let mut signing_keys = vec![];
        for signer in &self.settings.signers {
            let public_key = wallet
                .find_public_key(&signer.name)
                .map_err(|e| StepError::Wallet(e.to_string()))?;
            signing_keys.push(public_key)
        }
        init_account_builder = init_account_builder.signing_keys(signing_keys);
        drop(wallet);

        let (init_account_tx, signing_data) = init_account_builder
            .build(&sdk.namada)
            .await
            .map_err(|e| StepError::Build(e.to_string()))?;

        Ok((init_account_tx, vec![signing_data], init_account_builder.tx))
    }

    async fn build_checks(
        &self,
        _sdk: &Sdk,
        _retry_config: RetryConfig,
    ) -> Result<Vec<Check>, StepError> {
        Ok(vec![Check::AccountExist(
            self.target.clone(),
            self.threshold,
            self.sources.clone(),
        )])
    }

    fn update_state(&self, state: &mut State, with_fee: bool) {
        if with_fee {
            state.modify_balance_fee(&self.settings.gas_payer, self.settings.gas_limit);
        }
        state.add_enstablished_account(&self.target, &self.sources, self.threshold);
    }
}
