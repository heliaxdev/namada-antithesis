use crate::{constants::DEFAULT_GAS_LIMIT, entities::Alias};

#[derive(Clone, Debug)]
pub struct TaskSettings {
    pub signers: Vec<Alias>,
    pub gas_payer: Alias,
    pub gas_limit: u64,
}

impl TaskSettings {
    pub fn new(signers: Vec<Alias>, gas_payer: Alias) -> Self {
        Self {
            signers,
            gas_payer,
            gas_limit: DEFAULT_GAS_LIMIT,
        }
    }
}

pub type Target = Alias;
pub type Source = Alias;
pub type Amount = u64;

#[derive(Clone, Debug)]
pub enum Task {
    NewWalletKeyPair(Source),
    FaucetTransfer(Target, Amount, TaskSettings),
    TransparentTransfer(Source, Target, Amount, TaskSettings),
}
