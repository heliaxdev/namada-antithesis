use namada_sdk::args::{self, TxBuilder};
use namada_sdk::signing::SigningTxData;
use namada_sdk::tx::data::GasLimit;
use namada_sdk::tx::Tx;
use namada_sdk::Namada;
use typed_builder::TypedBuilder;

use crate::check::{self, Check};
use crate::context::Ctx;
use crate::error::TaskError;
use crate::state::State;
use crate::task::{TaskContext, TaskSettings};
use crate::types::{Alias, ProposalId, ProposalVote};
use crate::utils::RetryConfig;

#[derive(Clone, Debug, TypedBuilder)]
pub struct Vote {
    source: Alias,
    proposal_id: ProposalId,
    vote: ProposalVote,
    settings: TaskSettings,
}

impl TaskContext for Vote {
    fn name(&self) -> String {
        "vote".to_string()
    }

    fn summary(&self) -> String {
        format!(
            "vote/{}/{}/{}",
            self.source.name, self.proposal_id, self.vote
        )
    }

    fn task_settings(&self) -> Option<&TaskSettings> {
        Some(&self.settings)
    }

    async fn build_tx(&self, ctx: &Ctx) -> Result<(Tx, Vec<SigningTxData>, args::Tx), TaskError> {
        let wallet = ctx.namada.wallet.read().await;
        let source_address = wallet
            .find_address(&self.source.name)
            .ok_or_else(|| TaskError::Wallet(format!("No source address: {}", self.source.name)))?;
        let fee_payer = wallet
            .find_public_key(&self.settings.gas_payer.name)
            .map_err(|e| TaskError::Wallet(e.to_string()))?;

        let mut vote_tx_builder = ctx.namada.new_proposal_vote(
            self.proposal_id,
            self.vote.to_string(),
            source_address.into_owned(),
        );
        vote_tx_builder = vote_tx_builder.gas_limit(GasLimit::from(self.settings.gas_limit));
        vote_tx_builder = vote_tx_builder.wrapper_fee_payer(fee_payer);
        let mut signing_keys = vec![];
        for signer in &self.settings.signers {
            let public_key = wallet
                .find_public_key(&signer.name)
                .map_err(|e| TaskError::Wallet(e.to_string()))?;
            signing_keys.push(public_key)
        }
        vote_tx_builder = vote_tx_builder.signing_keys(signing_keys);
        drop(wallet);

        let (vote_tx, signing_data) = vote_tx_builder
            .build(&ctx.namada)
            .await
            .map_err(|e| TaskError::BuildTx(e.to_string()))?;

        Ok((vote_tx, vec![signing_data], vote_tx_builder.tx))
    }

    async fn build_checks(
        &self,
        _ctx: &Ctx,
        _retry_config: RetryConfig,
    ) -> Result<Vec<Check>, TaskError> {
        Ok(vec![Check::VoteResult(
            check::vote_result::VoteResult::builder()
                .source(self.source.clone())
                .proposal_id(self.proposal_id)
                .vote(self.vote.clone())
                .build(),
        )])
    }

    fn update_state(&self, _state: &mut State) {}
}
