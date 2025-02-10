use std::{
    collections::{BTreeSet, HashMap, HashSet},
    env,
    fs::{self},
    path::PathBuf,
};

use rand::{seq::IteratorRandom, Rng, RngCore};
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
    pub fn is_enstablished(&self) -> bool {
        matches!(self, AddressType::Enstablished)
    }
}

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub alias: Alias,
    pub public_keys: BTreeSet<Alias>,
    pub threshold: u64,
    pub address_type: AddressType,
}

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaspAccount {
    pub alias: Alias,
    pub spending_key: Alias,
    pub payment_address: Alias,
}

impl Account {
    pub fn is_implicit(&self) -> bool {
        self.address_type.is_implicit()
    }
    pub fn is_enstablished(&self) -> bool {
        self.address_type.is_enstablished()
    }
}

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bond {
    pub alias: Alias,
    pub validator: String,
    pub amount: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    pub accounts: HashMap<Alias, Account>,
    pub masp_accounts: HashMap<Alias, MaspAccount>,
    pub balances: HashMap<Alias, u64>,
    pub masp_balances: HashMap<Alias, u64>, // why do we have masp balances if tha aliases for masp accounts has a sufix?
    pub bonds: HashMap<Alias, HashMap<String, u64>>,
    pub unbonds: HashMap<Alias, HashMap<String, u64>>,
    pub redelegations: HashMap<Alias, HashMap<String, u64>>,
    pub validators: HashMap<Alias, Account>,
    pub seed: u64,
    pub rng: AntithesisRng,
    pub path: PathBuf,
    pub base_dir: PathBuf,
    pub stats: HashMap<String, u64>,
}

impl State {
    pub fn new(id: u64, seed: u64) -> Self {
        Self {
            accounts: HashMap::default(),
            masp_accounts: HashMap::default(),
            balances: HashMap::default(),
            masp_balances: HashMap::default(),
            bonds: HashMap::default(),
            unbonds: HashMap::default(),
            redelegations: HashMap::default(),
            validators: HashMap::default(),
            seed,
            rng: AntithesisRng::default(),
            path: env::current_dir()
                .unwrap()
                .join(format!("state-{}.json", id)),
            base_dir: env::current_dir().unwrap().join("base"),
            stats: HashMap::default(),
        }
    }

    pub fn update(&mut self, tasks: Vec<Task>, with_fee: bool) {
        for task in tasks {
            match task.clone() {
                Task::NewWalletKeyPair(alias) => {
                    self.add_implicit_account(alias.clone());
                    self.add_masp_account(alias);
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
                    if with_fee {
                        self.modify_balance_fee(setting.gas_payer, setting.gas_limit);
                    }
                    self.modify_redelegate(source, from, to, amount)
                }
                Task::Batch(tasks, setting) => {
                    self.modify_balance_fee(setting.gas_payer, setting.gas_limit);
                    self.update(tasks, false);
                }
                Task::Unbond(source, validator, amount, _epoch, setting) => {
                    if with_fee {
                        self.modify_balance_fee(setting.gas_payer, setting.gas_limit);
                    }
                    self.modify_unbonds(source, validator, amount);
                }
                Task::ClaimRewards(_source, _validator, setting) => {
                    if with_fee {
                        self.modify_balance_fee(setting.gas_payer, setting.gas_limit);
                    }
                }
                Task::InitAccount(alias, sources, threshold, setting) => {
                    if with_fee {
                        self.modify_balance_fee(setting.gas_payer, setting.gas_limit);
                    }
                    self.add_enstablished_account(alias, sources, threshold);
                }
                Task::ShieldedTransfer(source, target, amount, setting) => {
                    if with_fee {
                        self.modify_balance_fee(setting.gas_payer, setting.gas_limit);
                    }
                    self.modify_shielded_transfer(source, target, amount);
                }
                Task::Shielding(source, target, amount, setting) => {
                    if with_fee {
                        self.modify_balance_fee(setting.gas_payer, setting.gas_limit);
                    }
                    self.modify_shielding(source, target, amount)
                }
                Task::Unshielding(source, target, amount, setting) => {
                    if with_fee {
                        self.modify_balance_fee(setting.gas_payer, setting.gas_limit);
                    }
                    self.modify_unshielding(source, target, amount)
                }
                Task::BecomeValidator(alias, _, _, _, _, _, _, setting) => {
                    if with_fee {
                        self.modify_balance_fee(setting.gas_payer, setting.gas_limit);
                    }
                    self.set_enstablished_as_validator(alias)
                }
                Task::ChangeMetadata(_, _, _, _, _, _, setting) => {
                    if with_fee {
                        self.modify_balance_fee(setting.gas_payer, setting.gas_limit);
                    }
                }
                Task::ChangeConsensusKeys(_, _, setting) => {
                    if with_fee {
                        self.modify_balance_fee(setting.gas_payer, setting.gas_limit);
                    }
                }
                Task::UpdateAccount(target, sources, threshold, setting) => {
                    if with_fee {
                        self.modify_balance_fee(setting.gas_payer, setting.gas_limit);
                    }
                    self.modify_enstablished_account(target, sources, threshold);
                }
            }
            self.stats
                .entry(task.raw_type())
                .and_modify(|counter| *counter += 1)
                .or_insert(1);
        }
    }

    pub fn update_failed_execution(&mut self, tasks: &[Task]) {
        for task in tasks {
            let settings = match task {
                Task::NewWalletKeyPair(_alias) => None,
                Task::FaucetTransfer(_alias, _, task_settings) => Some(task_settings),
                Task::TransparentTransfer(_alias, _alias1, _, task_settings) => Some(task_settings),
                Task::Bond(_alias, _, _, _, task_settings) => Some(task_settings),
                Task::Unbond(_alias, _, _, _, task_settings) => Some(task_settings),
                Task::Redelegate(_alias, _, _, _, _, task_settings) => Some(task_settings),
                Task::ClaimRewards(_alias, _, task_settings) => Some(task_settings),
                Task::Batch(_tasks, task_settings) => Some(task_settings),
                Task::Shielding(_alias, _alias1, _, task_settings) => Some(task_settings),
                Task::InitAccount(_alias, _btree_set, _, task_settings) => Some(task_settings),
                Task::BecomeValidator(
                    _alias,
                    _alias1,
                    _alias2,
                    _alias3,
                    _alias4,
                    _dec,
                    _dec1,
                    task_settings,
                ) => Some(task_settings),
                Task::ShieldedTransfer(_, _, _, task_settings) => Some(task_settings),
                Task::Unshielding(_, _, _, task_settings) => Some(task_settings),
                Task::ChangeMetadata(_, _, _, _, _, _, task_settings) => Some(task_settings),
                Task::ChangeConsensusKeys(_, _, task_settings) => Some(task_settings),
                Task::UpdateAccount(_, _, _, task_settings) => Some(task_settings),
            };
            if let Some(settings) = settings {
                self.modify_balance_fee(settings.gas_payer.clone(), settings.gas_limit);
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
                    tracing::warn!("Can't deserialize state file, creating new one.");
                    let state = State::new(
                        id,
                        seed.unwrap_or(rand::thread_rng().gen_range(0..u64::MAX)),
                    );
                    state.serialize_to_file();
                    state
                }
            },
            Err(_) => {
                tracing::warn!("Can't read state file, creating new one.");
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

    pub fn at_least_masp_accounts(&self, sample: u64) -> bool {
        self.masp_accounts.len() >= sample as usize
    }

    pub fn at_least_masp_account_with_minimal_balance(
        &self,
        number_of_accounts: usize,
        min_balance: u64,
    ) -> bool {
        self.masp_accounts
            .iter()
            .filter(|(_, account)| {
                self.get_shielded_balance_for(&account.payment_address) >= min_balance
            })
            .count()
            >= number_of_accounts
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

    pub fn min_n_enstablished_accounts(&self, sample_size: usize) -> bool {
        self.accounts
            .iter()
            .filter(|(_, account)| account.is_enstablished())
            .count()
            > sample_size
    }

    pub fn any_bond(&self) -> bool {
        self.min_bonds(1)
    }

    pub fn min_bonds(&self, sample: usize) -> bool {
        self.bonds
            .values()
            .filter(|data| data.values().any(|data| *data > 2))
            .flatten()
            .count()
            >= sample
    }

    pub fn min_n_validators(&self, sample: usize) -> bool {
        self.validators.len() >= sample
    }

    /// GET

    pub fn random_account(&mut self, blacklist: Vec<Alias>) -> Option<Account> {
        self.accounts
            .iter()
            .filter(|(alias, _)| !blacklist.contains(alias))
            .choose(&mut self.rng)
            .map(|(_, account)| account.clone())
    }

    pub fn random_masp_account_with_min_balance(
        &mut self,
        blacklist: Vec<Alias>,
        min_value: u64,
    ) -> Option<MaspAccount> {
        self.masp_balances
            .iter()
            .filter_map(|(alias, balance)| {
                if blacklist.contains(alias) {
                    return None;
                }
                if balance >= &min_value {
                    Some(self.masp_accounts.get(alias).unwrap().clone())
                } else {
                    None
                }
            })
            .choose(&mut self.rng)
    }

    pub fn random_payment_address(&mut self, blacklist: Vec<Alias>) -> Option<MaspAccount> {
        self.masp_accounts
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
            .filter(|(_, account)| account.is_implicit())
            .choose_multiple(&mut self.rng, sample_size)
            .into_iter()
            .map(|(_, account)| account.clone())
            .collect()
    }

    pub fn random_enstablished_account(
        &mut self,
        blacklist: Vec<Alias>,
        sample_size: usize,
    ) -> Vec<Account> {
        self.accounts
            .iter()
            .filter(|(alias, _)| !blacklist.contains(alias))
            .filter(|(_, account)| account.is_enstablished())
            .choose_multiple(&mut self.rng, sample_size)
            .into_iter()
            .map(|(_, account)| account.clone())
            .collect()
    }

    pub fn random_validator(&mut self, blacklist: Vec<Alias>, sample_size: usize) -> Vec<Account> {
        self.validators
            .iter()
            .filter(|(alias, _)| !blacklist.contains(alias))
            .filter(|(_, account)| account.is_enstablished())
            .choose_multiple(&mut self.rng, sample_size)
            .into_iter()
            .map(|(_, account)| account.clone())
            .collect()
    }

    pub fn random_bond(&mut self) -> Bond {
        self.bonds
            .iter()
            .flat_map(|(source, bonds)| {
                bonds.iter().filter_map(|(validator, amount)| {
                    if *amount > 1 {
                        Some(Bond {
                            alias: source.to_owned(),
                            validator: validator.to_owned(),
                            amount: *amount,
                        })
                    } else {
                        None
                    }
                })
            })
            .choose(&mut self.rng)
            .unwrap()
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

    pub fn get_shielded_balance_for(&self, alias: &Alias) -> u64 {
        let stripped_alias = Alias {
            name: alias
                .name
                .strip_suffix("-payment-address")
                .unwrap()
                .to_string(),
        };
        self.masp_balances
            .get(&stripped_alias)
            .cloned()
            .unwrap_or_default()
    }

    pub fn get_redelegations_targets_for(&mut self, alias: &Alias) -> HashSet<String> {
        self.redelegations
            .get(alias)
            .map(|data| data.keys().cloned().collect::<HashSet<String>>())
            .unwrap_or_default()
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

    pub fn add_masp_account(&mut self, alias: Alias) {
        self.masp_accounts.insert(
            alias.clone(),
            MaspAccount {
                alias: alias.clone(),
                spending_key: format!("{}-spending-key", alias.name).into(),
                payment_address: format!("{}-payment-address", alias.name).into(),
            },
        );
        self.masp_balances.insert(alias.clone(), 0);
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

    pub fn modify_enstablished_account(
        &mut self,
        alias: Alias,
        aliases: BTreeSet<Alias>,
        threshold: u64,
    ) {
        assert!(self.accounts.contains_key(&alias));
        self.accounts.entry(alias).and_modify(|account| {
            account.public_keys = aliases;
            account.threshold = threshold;
        });
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

    pub fn modify_redelegate(&mut self, source: Alias, from: String, to: String, amount: u64) {
        let default = HashMap::from_iter([(to.clone(), 0u64)]);
        *self
            .redelegations
            .entry(source.clone())
            .or_insert(default)
            .entry(to)
            .or_insert(0) += amount;
        self.bonds
            .entry(source.clone())
            .and_modify(|bond| *bond.get_mut(&from).unwrap() -= amount);
    }

    pub fn modify_unbonds(&mut self, source: Alias, validator: String, amount: u64) {
        let default = HashMap::from_iter([(validator.clone(), 0u64)]);
        *self
            .unbonds
            .entry(source.clone())
            .or_insert(default)
            .entry(validator.clone())
            .or_insert(0) += amount;
        self.bonds
            .entry(source.clone())
            .and_modify(|bond| *bond.get_mut(&validator).unwrap() -= amount);
    }

    pub fn modify_shielding(&mut self, source: Alias, target: Alias, amount: u64) {
        *self.balances.get_mut(&source).unwrap() -= amount;
        let target_alias = Alias {
            name: target
                .name
                .strip_suffix("-payment-address")
                .unwrap()
                .to_string(),
        };
        *self.masp_balances.get_mut(&target_alias).unwrap() += amount;
    }

    pub fn modify_unshielding(&mut self, source: Alias, target: Alias, amount: u64) {
        let source_alias = Alias {
            name: source
                .name
                .strip_suffix("-spending-key")
                .unwrap()
                .to_string(),
        };

        *self.masp_balances.get_mut(&source_alias).unwrap() -= amount;
        *self.balances.get_mut(&target).unwrap() += amount;
    }

    pub fn modify_shielded_transfer(&mut self, source: Alias, target: Alias, amount: u64) {
        let target_alias = Alias {
            name: target
                .name
                .strip_suffix("-payment-address")
                .unwrap()
                .to_string(),
        };
        *self.masp_balances.get_mut(&target_alias).unwrap() += amount;

        let source_alias = Alias {
            name: source
                .name
                .strip_suffix("-spending-key")
                .unwrap()
                .to_string(),
        };
        *self.masp_balances.get_mut(&source_alias).unwrap() -= amount;
    }

    pub fn set_enstablished_as_validator(&mut self, alias: Alias) {
        let account = self.accounts.remove(&alias).unwrap();
        self.validators.insert(alias, account);
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AntithesisRng {}

impl RngCore for AntithesisRng {
    fn next_u32(&mut self) -> u32 {
        (antithesis_sdk::random::get_random() & 0xFFFF_FFFF) as u32
    }

    fn next_u64(&mut self) -> u64 {
        antithesis_sdk::random::get_random()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        let mut i = 0;
        while i + 8 <= dest.len() {
            let random = antithesis_sdk::random::get_random();
            dest[i..i + 8].copy_from_slice(&random.to_le_bytes());
            i += 8;
        }
        if i < dest.len() {
            let random = antithesis_sdk::random::get_random();
            let dest_len = dest.len();
            dest[i..].copy_from_slice(&random.to_le_bytes()[..dest_len - i]);
        }
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        self.fill_bytes(dest);
        Ok(())
    }
}
