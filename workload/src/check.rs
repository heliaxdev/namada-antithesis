use std::fmt::{Display, Formatter};

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

impl Display for Check {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Check::RevealPk(alias) => write!(f, "reveal-{}", alias.name),
            Check::BalanceSource(target, _pre_balance, _amount, _) => {
                write!(f, "balance-source-{}", target.name)
            }
            Check::BalanceTarget(target, _pre_balance, _amount, _) => {
                write!(f, "balance-target-{}", target.name)
            }
            Check::Bond(source, validator, _pre_balance, _amount, _) => {
                write!(f, "bond-{}-{}", source.name, validator)
            }
        }
    }
}
