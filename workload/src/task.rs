use std::{collections::BTreeSet, fmt::Display};

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
            gas_limit: DEFAULT_GAS_LIMIT,
        }
    }

    pub fn faucet() -> Self {
        Self {
            signers: BTreeSet::from_iter(vec![Alias::faucet()]),
            gas_payer: Alias::faucet(),
            gas_limit: DEFAULT_GAS_LIMIT,
        }
    }

    pub fn faucet_batch(size: usize) -> Self {
        Self {
            signers: BTreeSet::from_iter(vec![Alias::faucet()]),
            gas_payer: Alias::faucet(),
            gas_limit: DEFAULT_GAS_LIMIT * size as u64 * 10,
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

#[derive(Clone, Debug)]
pub enum Task {
    NewWalletKeyPair(Source),
    FaucetTransfer(Target, Amount, TaskSettings),
    TransparentTransfer(Source, Target, Amount, TaskSettings),
    Bond(Source, Address, Amount, Epoch, TaskSettings),
    Unbond(Source, Address, Amount, Epoch, TaskSettings),
    Redelegate(Source, Address, Address, Amount, Epoch, TaskSettings),
    Batch(Vec<Task>, TaskSettings),
    Shielding(Source, PaymentAddress, Amount, TaskSettings),
    InitAccount(Source, BTreeSet<Source>, Threshold, TaskSettings),
}

impl Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Task::NewWalletKeyPair(source) => write!(f, "wallet-key-pair/{}", source.name),
            Task::FaucetTransfer(target, amount, _) => write!(f, "faucet-transfer/{}/{}", target.name, amount),
            Task::TransparentTransfer(source, target, amount, _) => {
                write!(f, "transparent-transfer/{}/{}/{}", source.name, target.name, amount)
            }
            Task::Bond(source, validator, amount, _, _) => {
                write!(f, "bond/{}/{}/{}", source.name, validator, amount)
            }
            Task::Unbond(source, validator, amount, _, _) => {
                write!(f, "unbond/{}/{}/{}", source.name, validator, amount)
            }
            Task::Shielding(source, target, amount, _) => {
                write!(f, "shielding/{}/{}/{}", source.name, target.name, amount)
            }
            Task::InitAccount(alias, _, _, _) => write!(f, "init-account/{}", alias.name),
            Task::Redelegate(source, from, to, amount, _, _) => write!(f, "redelegate/{}/{}/{}/{}", source.name, from, to, amount),
            Task::Batch(tasks, _) => {
                let tasks = tasks.iter().map(|task| task.to_string()).collect::<Vec<String>>();
                write!(f, "batch-{} -> {}", tasks.len(), tasks.join(" -> "))
            },
        }
    }
}
