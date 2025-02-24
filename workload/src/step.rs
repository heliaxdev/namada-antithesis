use std::fmt::Display;
use std::str::FromStr;

use enum_dispatch::enum_dispatch;

use crate::code::Code;
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::task::Task;

mod batch;
mod become_validator;
mod bond;
mod change_consensus_key;
mod change_metadata;
mod claim_rewards;
mod deactivate_validator;
mod default_proposal;
mod faucet_transfer;
mod init_account;
mod new_wallet_keypair;
mod reactivate_validator;
mod redelegate;
mod shielded_transfer;
mod shielding;
mod transparent_transfer;
mod unbond;
mod unshielding;
mod update_account;
mod utils;
mod vote;

#[enum_dispatch]
#[derive(Clone, Debug)]
pub enum StepType {
    NewWalletKeyPair(new_wallet_keypair::NewWalletKeyPair),
    FaucetTransfer(faucet_transfer::FaucetTransfer),
    TransparentTransfer(transparent_transfer::TransparentTransfer),
    Shielding(shielding::Shielding),
    Shielded(shielded_transfer::ShieldedTransfer),
    Unshielding(unshielding::Unshielding),
    Bond(bond::Bond),
    Unbond(unbond::Unbond),
    Redelegate(redelegate::Redelegate),
    ClaimRewards(claim_rewards::ClaimRewards),
    InitAccount(init_account::InitAccount),
    UpdateAccount(update_account::UpdateAccount),
    BecomeValidator(become_validator::BecomeValidator),
    DeactivateValidator(deactivate_validator::DeactivateValidator),
    ReactivateValidator(reactivate_validator::ReactivateValidator),
    ChangeMetadata(change_metadata::ChangeMetadata),
    ChangeConsensusKey(change_consensus_key::ChangeConsensusKey),
    DefaultProposal(default_proposal::DefaultProposal),
    Vote(vote::Vote),
    BatchBond(batch::BatchBond),
    BatchRandom(batch::BatchRandom),
}

impl FromStr for StepType {
    type Err = String;

    fn from_str(step: &str) -> Result<Self, Self::Err> {
        let step_type = match step {
            "new-wallet-key-pair" => Self::NewWalletKeyPair(Default::default()),
            "faucet-transfer" => Self::FaucetTransfer(Default::default()),
            "transparent-transfer" => Self::TransparentTransfer(Default::default()),
            "shielding" => Self::Shielding(Default::default()),
            "shielded" => Self::Shielded(Default::default()),
            "unshielding" => Self::Unshielding(Default::default()),
            "bond" => Self::Bond(Default::default()),
            "unbond" => Self::Unbond(Default::default()),
            "redelegate" => Self::Redelegate(Default::default()),
            "claim-rewards" => Self::ClaimRewards(Default::default()),
            "init-account" => Self::InitAccount(Default::default()),
            "update-account" => Self::UpdateAccount(Default::default()),
            "become-validator" => Self::BecomeValidator(Default::default()),
            "deactivate-validator" => Self::DeactivateValidator(Default::default()),
            "reactivate-validator" => Self::ReactivateValidator(Default::default()),
            "change-metadata" => Self::ChangeMetadata(Default::default()),
            "change-consensus-key" => Self::ChangeConsensusKey(Default::default()),
            "default-proposal" => Self::DefaultProposal(Default::default()),
            "vote" => Self::Vote(Default::default()),
            "batch-bond" => Self::BatchBond(Default::default()),
            "batch-random" => Self::BatchRandom(Default::default()),
            _ => return Err(format!("Invalid step type was given: {step}")),
        };

        Ok(step_type)
    }
}

impl Display for StepType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[enum_dispatch(StepType)]
pub trait StepContext {
    fn name(&self) -> String;

    #[allow(async_fn_in_trait)]
    async fn is_valid(&self, sdk: &Sdk, state: &State) -> Result<bool, StepError>;

    #[allow(async_fn_in_trait)]
    async fn build_task(&self, sdk: &Sdk, state: &mut State) -> Result<Vec<Task>, StepError>;

    fn assert(&self, code: &Code);
}
