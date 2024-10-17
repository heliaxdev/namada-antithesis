use std::collections::{BTreeSet, HashMap};

use rand::{seq::IteratorRandom, SeedableRng};
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{self, Read, Write};

use crate::{
    constants::{DEFAULT_FEE_IN_NATIVE_TOKEN, MIN_TRANSFER_BALANCE},
    entities::Alias,
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

#[derive(Clone, Debug, Serialize)]
pub struct State {
    pub accounts: HashMap<Alias, Account>,
    pub balances: HashMap<Alias, u64>,
    pub bonds: HashMap<Alias, HashMap<String, u64>>,
    #[serde(skip)]
    pub rng: ChaCha20Rng,
}


#[derive(Deserialize, Clone, Debug, Serialize)]
pub struct SaveState {
    pub accounts: HashMap<Alias, Account>,
    pub balances: HashMap<Alias, u64>,
    pub bonds: HashMap<Alias, HashMap<String, u64>>,
}

impl SaveState {
    pub fn from_state(s: &State) -> Self {
        SaveState {
            accounts: s.accounts.clone(),
            balances: s.balances.clone(),
            bonds: s.bonds.clone()
        }
    }

    pub fn to_state(ss: Self, seed: u64) -> State {
        State {
            accounts: ss.accounts,
            balances: ss.balances,
            bonds: ss.bonds,
            rng: ChaCha20Rng::seed_from_u64(seed),
        }
    }
}

impl State {
    pub fn new(seed: u64) -> Self {
        Self {
            accounts: HashMap::default(),
            balances: HashMap::default(),
            bonds: HashMap::default(),
            rng: ChaCha20Rng::seed_from_u64(seed),
        }
    }
    /// READ
    
    pub fn write_to_file(&self, file_path: &str) -> io::Result<()> {
        let ss = SaveState::from_state(self);
        let serialized_data = serde_json::to_string_pretty(&ss)
            .expect("Failed to serialize the State struct");
        let mut file = File::create(file_path)?;
        file.write_all(serialized_data.as_bytes())?;
        Ok(())
    }

    pub fn read_from_file(file_path: &str, seed: u64) -> io::Result<Self> {
        let mut file = File::open(file_path);
        match file {
            Ok(mut f) => {
                let mut buffer = String::new();
                f.read_to_string(&mut buffer)?;
                let mut res = serde_json::from_str(&buffer);
                match res {
                    Ok(ss) => Ok(SaveState::to_state(ss, seed)),
                    Err(e) => {
                        println!("Failed to deserialize: {:?}", e);
                        Err(e.into())
                    }
                }
            },
            Err(e) => {
                println!("File does not exist");
                Err(e)
            }
        }

    }

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

    /// GET

    pub fn random_account(&mut self, blacklist: Vec<Alias>) -> Account {
        self.accounts
            .iter()
            .filter(|(alias, _)| !blacklist.contains(alias))
            .choose(&mut self.rng)
            .map(|(_, account)| account.clone())
            .unwrap()
    }

    pub fn random_account_with_min_balance(&mut self, blacklist: Vec<Alias>) -> Account {
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
}
