use std::{
    collections::BTreeSet,
    fmt::{Display, Formatter},
};

use crate::{entities::Alias, state::State};

pub type Target = Alias;
pub type PreBalance = namada_sdk::token::Amount;
pub type Amount = u64;
pub type Address = String;
pub type Threshold = u64;

#[derive(Clone, Debug)]
pub enum Check {
    RevealPk(Target),
    BalanceTarget(Target, PreBalance, Amount, State),
    BalanceSource(Target, PreBalance, Amount, State),
    BalanceShieldedTarget(Target, PreBalance, Amount, State),
    BondIncrease(Target, Address, PreBalance, Amount, State),
    BondDecrease(Target, Address, PreBalance, Amount, State),
    AccountExist(Target, Threshold, BTreeSet<Target>, State),
}

impl Display for Check {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Check::RevealPk(alias) => write!(f, "reveal/{}", alias.name),
            Check::BalanceSource(target, _pre_balance, _amount, _) => {
                write!(f, "balance/source/{}", target.name)
            }
            Check::BalanceTarget(target, _pre_balance, _amount, _) => {
                write!(f, "balance/target/{}", target.name)
            }
            Check::BalanceShieldedTarget(target, _pre_balance, _amount, _) => {
                write!(f, "balance-shielded/target/{}", target.name)
            }
            Check::BondIncrease(source, validator, _pre_balance, _amount, _) => {
                write!(f, "bond/{}/{}/increase", source.name, validator)
            }
            Check::BondDecrease(source, validator, _pre_balance, _amount, _) => {
                write!(f, "bond/{}/{}/decrease", source.name, validator)
            }
            Check::AccountExist(source, _threshold, _sources, _) => {
                write!(f, "account-exist/{}", source.name)
            }
        }
    }
}
