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
pub enum ValidatorStatus {
    Active,
    Inactive,
}

impl Display for ValidatorStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidatorStatus::Active => write!(f, "active"),
            ValidatorStatus::Inactive => write!(f, "inactive"),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Check {
    RevealPk(Target),
    BalanceTarget(Target, PreBalance, Amount, State),
    BalanceSource(Target, PreBalance, Amount, State),
    BalanceShieldedTarget(Target, PreBalance, Amount, State),
    BalanceShieldedSource(Target, PreBalance, Amount, State),
    BondIncrease(Target, Address, PreBalance, Amount, State),
    BondDecrease(Target, Address, PreBalance, Amount, State),
    AccountExist(Target, Threshold, BTreeSet<Target>, State),
    IsValidatorAccount(Target),
    ValidatorStatus(Target, ValidatorStatus),
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
            Check::BalanceShieldedSource(target, _pre_balance, _amount, _) => {
                write!(f, "balance-shielded/source/{}", target.name)
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
            Check::IsValidatorAccount(target) => {
                write!(f, "is-validator/{}", target.name)
            }
            Check::ValidatorStatus(target, status) => {
                write!(f, "validator-status/{}/{}", target.name, status)
            }
        }
    }
}
