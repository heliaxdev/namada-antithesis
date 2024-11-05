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
            gas_limit: DEFAULT_GAS_LIMIT * size as u64,
        }
    }
}

pub type Target = Alias;
pub type Source = Alias;
pub type Amount = u64;
pub type Address = String;
pub type Epoch = u64;

#[derive(Clone, Debug)]
pub enum Task {
    NewWalletKeyPair(Source),
    FaucetTransfer(Target, Amount, TaskSettings),
    TransparentTransfer(Source, Target, Amount, TaskSettings),
    Bond(Source, Address, Amount, Epoch, TaskSettings),
    Batch(Vec<Task>, TaskSettings),
}

impl Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Task::NewWalletKeyPair(source) => write!(f, "wallet-key-pair-{}", source.name),
            Task::FaucetTransfer(target, _, _) => write!(f, "faucet-transfer-{}", target.name),
            Task::TransparentTransfer(source, target, _, _) => {
                write!(f, "transparent-transfer-{}-{}", source.name, target.name)
            }
            Task::Bond(source, validator, _, _, _) => {
                write!(f, "bond-{}-{}", source.name, validator)
            }
            Task::Batch(_, _) => write!(f, "batch"),
        }
    }
}
