use crate::{entities::Alias, state::State};

pub type Target = Alias;
pub type PreBalance = namada_sdk::token::Amount;
pub type Amount = u64;
pub type Address = String;

#[derive(Clone, Debug)]
pub enum Check {
    RevealPk(Target),
    BalanceTarget(Target, PreBalance, Amount, State),
    BalanceSource(Target, PreBalance, Amount, State),
    Bond(Target, Address, PreBalance, Amount, State),
}
