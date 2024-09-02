use std::collections::{BTreeSet, HashMap};

use rand::seq::IteratorRandom;

use crate::{
    constants::{DEFAULT_FEE_IN_NATIVE_TOKEN, DEFAULT_GAS_LIMIT, MIN_TRANSFER_BALANCE},
    entities::Alias,
};

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
pub enum AddressType {
    Enstablished,
    #[default]
    Implicit,
}

impl AddressType {
    pub fn is_implicit(&self) -> bool {
        matches!(self, AddressType::Implicit)
    }
}

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
pub struct Account {
    pub alias: Alias,
    pub public_keys: BTreeSet<Alias>,
    pub threshold: u64,
    pub address_type: AddressType,
}

impl Account {
    pub fn is_implicit(&self) -> bool {
        self.address_type.is_implicit()
    }
}

#[derive(Clone, Debug, Default)]
pub struct State {
    pub accounts: HashMap<Alias, Account>,
    pub balances: HashMap<Alias, u64>,
}

impl State {
    /// READ

    pub fn any_account(&self) -> bool {
        self.at_least_accounts(1)
    }

    pub fn at_least_accounts(&self, min_accounts: u64) -> bool {
        self.accounts.len() >= min_accounts as usize
    }

    pub fn any_account_with_min_balance(&self, min_balance: u64) -> bool {
        self.balances
            .iter()
            .any(|(_, balance)| balance >= &min_balance)
    }

    pub fn any_account_can_pay_fees(&self) -> bool {
        self.balances.iter().any(|(alias, balance)| {
            if balance >= &DEFAULT_GAS_LIMIT {
                let account = self.accounts.get(alias).expect("Alias should exist.");
                account.is_implicit()
            } else {
                false
            }
        })
    }
    pub fn any_account_can_make_transfer(&self) -> bool {
        self.balances
            .iter()
            .any(|(_, balance)| balance >= &MIN_TRANSFER_BALANCE)
    }

    /// GET

    pub fn random_account(&self) -> Account {
        self.accounts
            .iter()
            .choose(&mut rand::thread_rng())
            .map(|(_, account)| account.clone())
            .unwrap()
    }

    pub fn random_account_with_min_balance(&self, blacklist: Vec<Alias>) -> Account {
        self.balances
            .iter()
            .filter_map(|(alias, balance)| {
                if blacklist.contains(alias) {
                    return None;
                }
                if balance >= &DEFAULT_GAS_LIMIT {
                    Some(self.accounts.get(alias).unwrap().clone())
                } else {
                    None
                }
            })
            .choose(&mut rand::thread_rng())
            .unwrap()
    }

    pub fn get_balance_for(&self, alias: &Alias) -> u64 {
        self.balances.get(alias).cloned().unwrap_or_default()
    }

    /// UPDATE

    pub fn add_implicit_account(&mut self, alias: Alias) {
        self.accounts.insert(
            alias.clone(),
            Account {
                alias: alias.clone(),
                public_keys: BTreeSet::from_iter(vec![alias.clone()]),
                threshold: 1,
                address_type: AddressType::Implicit,
            },
        );
        self.balances.insert(alias, 0);
    }

    pub fn modify_balance(&mut self, source: Alias, target: Alias, amount: u64) {
        if !source.is_faucet() {
            *self.balances.get_mut(&source).unwrap() -= amount;
        } 
        *self.balances.get_mut(&target).unwrap() += amount;
    }

    pub fn modify_balance_fee(&mut self, source: Alias, gas_limit: u64) {
        if !source.is_faucet() {
            *self.balances.get_mut(&source).unwrap() -= DEFAULT_FEE_IN_NATIVE_TOKEN;
        } 
        *self.balances.get_mut(&source).unwrap() += DEFAULT_FEE_IN_NATIVE_TOKEN;
    }
}
