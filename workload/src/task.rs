use std::{collections::BTreeSet, fmt::Display};

use namada_sdk::dec::Dec;

use crate::{constants::DEFAULT_GAS_LIMIT, entities::Alias};

#[derive(Clone, Debug)]
pub struct TaskSettings {
    pub signers: BTreeSet<Alias>,
    pub gas_payer: Alias,
    pub gas_limit: u64,
}

impl TaskSettings {
    pub fn new(signers: BTreeSet<Alias>, gas_payer: Alias) -> Self {
        Self {
            signers,
            gas_payer,
            gas_limit: DEFAULT_GAS_LIMIT * 2,
        }
    }

    pub fn faucet() -> Self {
        Self {
            signers: BTreeSet::from_iter(vec![Alias::faucet()]),
            gas_payer: Alias::faucet(),
            gas_limit: DEFAULT_GAS_LIMIT * 2,
        }
    }

    pub fn faucet_batch(size: usize) -> Self {
        Self {
            signers: BTreeSet::from_iter(vec![Alias::faucet()]),
            gas_payer: Alias::faucet(),
            gas_limit: DEFAULT_GAS_LIMIT * size as u64 * 2,
        }
    }
}

pub type Target = Alias;
pub type Source = Alias;
pub type PaymentAddress = Alias;
pub type Amount = u64;
pub type Address = String;
pub type Epoch = u64;
pub type Threshold = u64;
pub type WalletAlias = Alias;
pub type CommissionRate = Dec;
pub type CommissionChange = Dec;

#[derive(Clone, Debug)]
pub enum Task {
    NewWalletKeyPair(Source),
    FaucetTransfer(Target, Amount, TaskSettings),
    TransparentTransfer(Source, Target, Amount, TaskSettings),
    Bond(Source, Address, Amount, Epoch, TaskSettings),
    Unbond(Source, Address, Amount, Epoch, TaskSettings),
    Redelegate(Source, Address, Address, Amount, Epoch, TaskSettings),
    ClaimRewards(Source, Address, TaskSettings),
    Batch(Vec<Task>, TaskSettings),
    ShieldedTransfer(Source, Target, Amount, TaskSettings),
    Shielding(Source, PaymentAddress, Amount, TaskSettings),
    InitAccount(Source, BTreeSet<Source>, Threshold, TaskSettings),
    Unshielding(PaymentAddress, Target, Amount, TaskSettings),
    BecomeValidator(
        Source,
        WalletAlias,
        WalletAlias,
        WalletAlias,
        WalletAlias,
        CommissionRate,
        CommissionChange,
        TaskSettings,
    ),
    ChangeMetadata(Source, String, String, String, String, String, TaskSettings),
    ChangeConsensusKeys(Source, Alias, TaskSettings),
    DeactivateValidator(Source, TaskSettings),
    ReactivateValidator(Source, TaskSettings),
    UpdateAccount(Source, BTreeSet<Source>, Threshold, TaskSettings),
}

impl Task {
    pub fn raw_type(&self) -> String {
        match self {
            Task::NewWalletKeyPair(_) => "new-wallet-keypair".to_string(),
            Task::FaucetTransfer(_, _, _) => "faucet-transfer".to_string(),
            Task::TransparentTransfer(_, _, _, _) => "transparent-transfer".to_string(),
            Task::Bond(_, _, _, _, _) => "bond".to_string(),
            Task::Unbond(_, _, _, _, _) => "unbond".to_string(),
            Task::Redelegate(_, _, _, _, _, _) => "relegate".to_string(),
            Task::ClaimRewards(_, _, _) => "claim-rewards".to_string(),
            Task::Batch(_, _) => "batch".to_string(),
            Task::Shielding(_, _, _, _) => "shielding".to_string(),
            Task::Unshielding(_, _, _, _) => "unshielding".to_string(),
            Task::ShieldedTransfer(_, _, _, _) => "shielded-transfer".to_string(),
            Task::InitAccount(_, _, _, _) => "init-account".to_string(),
            Task::BecomeValidator(_, _, _, _, _, _, _, _) => "become-validator".to_string(),
            Task::ChangeMetadata(_, _, _, _, _, _, _) => "change-metadata".to_string(),
            Task::ChangeConsensusKeys(_, _, _) => "change-consensus-keys".to_string(),
            Task::UpdateAccount(_, _, _, _) => "update-account".to_string(),
            Task::DeactivateValidator(_, _) => "deactivate-validator".to_string(),
            Task::ReactivateValidator(_, _) => "reactivate-validator".to_string(),
        }
    }
}

impl Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Task::NewWalletKeyPair(source) => write!(f, "wallet-key-pair/{}", source.name),
            Task::FaucetTransfer(target, amount, _) => {
                write!(f, "faucet-transfer/{}/{}", target.name, amount)
            }
            Task::TransparentTransfer(source, target, amount, _) => {
                write!(
                    f,
                    "transparent-transfer/{}/{}/{}",
                    source.name, target.name, amount
                )
            }
            Task::Bond(source, validator, amount, _, _) => {
                write!(f, "bond/{}/{}/{}", source.name, validator, amount)
            }
            Task::Unbond(source, validator, amount, _, _) => {
                write!(f, "unbond/{}/{}/{}", source.name, validator, amount)
            }
            Task::ShieldedTransfer(source, target, amount, _) => {
                write!(
                    f,
                    "shielded-transfer/{}/{}/{}",
                    source.name, target.name, amount
                )
            }
            Task::Shielding(source, target, amount, _) => {
                write!(f, "shielding/{}/{}/{}", source.name, target.name, amount)
            }
            Task::Unshielding(source, target, amount, _) => {
                write!(f, "unshielding/{}/{}/{}", source.name, target.name, amount)
            }
            Task::InitAccount(alias, _, threshold, _) => {
                write!(f, "init-account/{}/{}", alias.name, threshold)
            }
            Task::BecomeValidator(alias, _, _, _, _, _, _, _) => {
                write!(f, "become-validator/{}", alias.name)
            }
            Task::ClaimRewards(alias, validator, _) => {
                write!(f, "claim-rewards/{}/{}", alias.name, validator)
            }
            Task::Redelegate(source, from, to, amount, _, _) => {
                write!(f, "redelegate/{}/{}/{}/{}", source.name, from, to, amount)
            }
            Task::ChangeMetadata(alias, _, _, _, _, _, _) => {
                write!(f, "change-metadata/{}", alias.name)
            }
            Task::ChangeConsensusKeys(alias, _, _) => {
                write!(f, "change-consensus-keys/{}", alias.name)
            }
            Task::UpdateAccount(source, _, _, _) => {
                write!(f, "update-account/{}", source.name)
            }
            Task::DeactivateValidator(source, _) => {
                write!(f, "deactivate-validator/{}", source.name)
            }
            Task::ReactivateValidator(source, _) => {
                write!(f, "deactivate-validator/{}", source.name)
            }
            Task::Batch(tasks, _) => {
                let tasks = tasks
                    .iter()
                    .map(|task| task.to_string())
                    .collect::<Vec<String>>();
                write!(f, "batch-{} -> {}", tasks.len(), tasks.join(" -> "))
            }
        }
    }
}
