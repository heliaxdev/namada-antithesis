use std::{
    collections::{BTreeSet, HashMap},
    env,
    fs::{self},
    path::PathBuf,
};

use rand::{seq::IteratorRandom, Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};

use crate::{
    constants::{DEFAULT_FEE_IN_NATIVE_TOKEN, MIN_TRANSFER_BALANCE},
    entities::Alias,
    task::Task,
};

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bond {
    pub alias: Alias,
    pub validator: String,
    pub amount: u64
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    pub accounts: HashMap<Alias, Account>,
    pub balances: HashMap<Alias, u64>,
    pub bonds: HashMap<Alias, HashMap<String, u64>>,
    pub redelegations: HashMap<Alias, HashMap<String, u64>>,
    pub seed: u64,
    pub rng: ChaCha20Rng,
    pub path: PathBuf,
    pub base_dir: PathBuf,
}

impl State {
    pub fn new(id: u64, seed: u64) -> Self {
        Self {
            accounts: HashMap::default(),
            balances: HashMap::default(),
            bonds: HashMap::default(),
            redelegations: HashMap::default(),
            seed,
            rng: ChaCha20Rng::seed_from_u64(seed),
            path: env::current_dir()
                .unwrap()
                .join(format!("state-{}.json", id)),
            base_dir: env::current_dir().unwrap().join("base"),
        }
    }

    pub fn update(&mut self, tasks: Vec<Task>, with_fee: bool) {
        for task in tasks {
            match task {
                Task::NewWalletKeyPair(alias) => {
                    self.add_implicit_account(alias);
                }
                Task::FaucetTransfer(target, amount, settings) => {
                    let source_alias = Alias::faucet();
                    self.modify_balance(source_alias, target, amount);
                    if with_fee {
                        self.modify_balance_fee(settings.gas_payer, settings.gas_limit);
                    }
                }
                Task::TransparentTransfer(source, target, amount, setting) => {
                    self.modify_balance(source, target, amount);
                    if with_fee {
                        self.modify_balance_fee(setting.gas_payer, setting.gas_limit);
                    }
                }
                Task::Bond(source, validator, amount, _, setting) => {
                    self.modify_bond(source, validator, amount);
                    if with_fee {
                        self.modify_balance_fee(setting.gas_payer, setting.gas_limit);
                    }
                }
                Task::Redelegate(source, from, to, amount, _epoch, setting) => {
                    self.modify_balance_fee(setting.gas_payer, setting.gas_limit);
                    self.modify_redeleagte(source, from, to, amount)
                }
                Task::Batch(tasks, setting) => {
                    self.modify_balance_fee(setting.gas_payer, setting.gas_limit);
                    self.update(tasks, false);
                }
                Task::InitAccount(alias, sources, threshold, setting) => {
                    self.add_enstablished_account(alias, sources, threshold);
                    if with_fee {
                        self.modify_balance_fee(setting.gas_payer, setting.gas_limit);
                    }
                }
            }
        }
    }

    pub fn serialize_to_file(&self) {
        fs::write(&self.path, serde_json::to_string_pretty(&self).unwrap()).unwrap()
    }

    pub fn from_file(id: u64, seed: Option<u64>) -> Self {
        let path = env::current_dir()
            .unwrap()
            .join(format!("state-{}.json", id));
        match fs::read_to_string(path) {
            Ok(data) => match serde_json::from_str(&data) {
                Ok(state) => state,
                Err(_) => {
                    let state = State::new(
                        id,
                        seed.unwrap_or(rand::thread_rng().gen_range(0..u64::MAX)),
                    );
                    state.serialize_to_file();
                    state
                }
            },
            Err(_) => {
                let state = State::new(
                    id,
                    seed.unwrap_or(rand::thread_rng().gen_range(0..u64::MAX)),
                );
                state.serialize_to_file();
                state
            }
        }
    }

    /// READ

    pub fn any_account(&self) -> bool {
        self.at_least_accounts(1)
    }

    pub fn at_least_accounts(&self, sample: u64) -> bool {
        self.accounts.len() >= sample as usize
    }

    pub fn any_account_with_min_balance(&self, min_balance: u64) -> bool {
        self.balances
            .iter()
            .any(|(_, balance)| balance >= &min_balance)
    }

    pub fn min_n_account_with_min_balance(&self, sample: usize, min_balance: u64) -> bool {
        self.balances
            .iter()
            .filter(|(_, balance)| **balance >= min_balance)
            .count()
            >= sample
    }

    pub fn any_account_can_pay_fees(&self) -> bool {
        self.balances.iter().any(|(alias, balance)| {
            if balance >= &DEFAULT_FEE_IN_NATIVE_TOKEN {
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

    pub fn min_n_implicit_accounts(&self, sample_size: usize) -> bool {
        self.accounts
            .iter()
            .filter(|(_, account)| account.is_implicit())
            .count()
            > sample_size
    }

    pub fn any_bond(&self) -> bool {
        self.min_bonds(1)
    }

    pub fn min_bonds(&self, sample: usize) -> bool {
        self.bonds.values().filter(|data| {
            data.values().any(|data| *data > 2)
        }).flatten().count() >= sample
    }

    /// GET

    pub fn random_account(&mut self, blacklist: Vec<Alias>) -> Option<Account> {
        self.accounts
            .iter()
            .filter(|(alias, _)| !blacklist.contains(alias))
            .choose(&mut self.rng)
            .map(|(_, account)| account.clone())
    }

    pub fn random_implicit_accounts(
        &mut self,
        blacklist: Vec<Alias>,
        sample_size: usize,
    ) -> Vec<Account> {
        self.accounts
            .iter()
            .filter(|(alias, _)| !blacklist.contains(alias))
            .choose_multiple(&mut self.rng, sample_size)
            .into_iter()
            .filter(|(_, account)| account.is_implicit())
            .map(|(_, account)| account.clone())
            .collect()
    }

    pub fn random_bond(
        &mut self,
    ) -> Bond {
        self.bonds
            .iter()
            .map(|(source, bonds)| {
                bonds.iter().map(|(validator, amount)| {
                    Bond {
                        alias: source.to_owned(),
                        validator: validator.to_owned(),
                        amount: *amount,
                    }
                }) 
            }).flatten().choose(&mut self.rng).unwrap()
    }

    pub fn random_account_with_min_balance(&mut self, blacklist: Vec<Alias>) -> Option<Account> {
        self.balances
            .iter()
            .filter_map(|(alias, balance)| {
                if blacklist.contains(alias) {
                    return None;
                }
                if balance >= &DEFAULT_FEE_IN_NATIVE_TOKEN {
                    Some(self.accounts.get(alias).unwrap().clone())
                } else {
                    None
                }
            })
            .choose(&mut self.rng)
    }

    pub fn get_account_by_alias(&self, alias: &Alias) -> Account {
        self.accounts.get(alias).unwrap().to_owned()
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
        self.balances.insert(alias.clone(), 0);
    }

    pub fn add_enstablished_account(
        &mut self,
        alias: Alias,
        aliases: BTreeSet<Alias>,
        threshold: u64,
    ) {
        self.accounts.insert(
            alias.clone(),
            Account {
                alias: alias.clone(),
                public_keys: aliases,
                threshold,
                address_type: AddressType::Enstablished,
            },
        );
        self.balances.insert(alias.clone(), 0);
    }

    pub fn modify_balance(&mut self, source: Alias, target: Alias, amount: u64) {
        if !source.is_faucet() {
            *self.balances.get_mut(&source).unwrap() -= amount;
        }
        *self.balances.get_mut(&target).unwrap() += amount;
    }

    pub fn modify_balance_fee(&mut self, source: Alias, _gas_limit: u64) {
        if !source.is_faucet() {
            *self.balances.get_mut(&source).unwrap() -= DEFAULT_FEE_IN_NATIVE_TOKEN;
        }
    }

    pub fn modify_bond(&mut self, source: Alias, validator: String, amount: u64) {
        if !source.is_faucet() {
            *self.balances.get_mut(&source).unwrap() -= amount;
        }
        let default = HashMap::from_iter([(validator.clone(), 0u64)]);
        *self
            .bonds
            .entry(source.clone())
            .or_insert(default)
            .entry(validator)
            .or_insert(0) += amount;
    }

    pub fn modify_redeleagte(&mut self, source: Alias, from: String, to: String, amount: u64) {
        let default = HashMap::from_iter([(to.clone(), 0u64)]);
        *self
            .redelegations
            .entry(source.clone())
            .or_insert(default)
            .entry(to)
            .or_insert(0) += amount;
        self
            .bonds
            .entry(source.clone())
            .and_modify(|bond| *bond.get_mut(&from).unwrap() -= amount);
    }
}
