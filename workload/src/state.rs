use std::{
    collections::{BTreeSet, HashMap, HashSet},
    env, fs,
    path::PathBuf,
};

use fs2::FileExt;
use rand::{seq::IteratorRandom, Rng, RngCore};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    constants::{DEFAULT_FEE_IN_NATIVE_TOKEN, MIN_TRANSFER_BALANCE, PROPOSAL_DEPOSIT},
    entities::Alias,
    task::Task,
};

#[derive(Error, Debug)]
pub enum StateError {
    #[error("File error: `{0}`")]
    File(std::io::Error),
    #[error("Encode/Decode error: `{0}`")]
    Serde(serde_json::Error),
    #[error("State file is empty")]
    EmptyFile,
}

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
    pub masp_balances: HashMap<Alias, u64>,
    pub bonds: HashMap<Alias, HashMap<String, u64>>,
    pub unbonds: HashMap<Alias, HashMap<String, u64>>,
    pub redelegations: HashMap<Alias, HashMap<String, u64>>,
    pub validators: HashMap<Alias, Account>,
    pub deactivated_validators: HashMap<Alias, Account>,
    pub proposals: HashMap<u64, (u64, u64)>,
    pub id: u64,
    pub seed: u64,
    pub rng: AntithesisRng,
    pub base_dir: PathBuf,
    pub stats: HashMap<String, u64>,
}

impl State {
    pub fn new(id: u64, seed: Option<u64>) -> Self {
        let seed = seed.unwrap_or(rand::thread_rng().gen_range(0..u64::MAX));
        Self {
            accounts: HashMap::default(),
            masp_accounts: HashMap::default(),
            balances: HashMap::default(),
            masp_balances: HashMap::default(),
            bonds: HashMap::default(),
            unbonds: HashMap::default(),
            redelegations: HashMap::default(),
            validators: HashMap::default(),
            deactivated_validators: HashMap::default(),
            proposals: HashMap::default(),
            id,
            seed,
            rng: AntithesisRng::default(),
            base_dir: env::current_dir().unwrap().join("base"),
            stats: HashMap::default(),
        }
    }

    pub fn update(&mut self, tasks: &Vec<Task>, with_fee: bool) {
        for task in tasks {
            match task {
                Task::NewWalletKeyPair(alias) => {
                    self.add_implicit_account(alias);
                    self.add_masp_account(alias);
                }
                Task::FaucetTransfer(target, amount, settings) => {
                    if with_fee {
                        self.modify_balance_fee(&settings.gas_payer, settings.gas_limit);
                    }
                    self.increase_balance(target, *amount);
                }
                Task::TransparentTransfer(source, target, amount, setting) => {
                    if with_fee {
                        self.modify_balance_fee(&setting.gas_payer, setting.gas_limit);
                    }
                    self.decrease_balance(source, *amount);
                    self.increase_balance(target, *amount);
                }
                Task::Bond(source, validator, amount, _, setting) => {
                    self.modify_bond(source, validator, *amount);
                    if with_fee {
                        self.modify_balance_fee(&setting.gas_payer, setting.gas_limit);
                    }
                }
                Task::Redelegate(source, from, to, amount, _epoch, setting) => {
                    if with_fee {
                        self.modify_balance_fee(&setting.gas_payer, setting.gas_limit);
                    }
                    self.modify_redelegate(source, from, to, *amount)
                }
                Task::Batch(tasks, setting) => {
                    self.modify_balance_fee(&setting.gas_payer, setting.gas_limit);
                    self.update(&tasks, false);
                }
                Task::Unbond(source, validator, amount, _epoch, setting) => {
                    if with_fee {
                        self.modify_balance_fee(&setting.gas_payer, setting.gas_limit);
                    }
                    self.modify_unbonds(source, validator, *amount);
                }
                Task::ClaimRewards(_source, _validator, setting) => {
                    if with_fee {
                        self.modify_balance_fee(&setting.gas_payer, setting.gas_limit);
                    }
                }
                Task::InitAccount(alias, sources, threshold, setting) => {
                    if with_fee {
                        self.modify_balance_fee(&setting.gas_payer, setting.gas_limit);
                    }
                    self.add_enstablished_account(alias, sources, *threshold);
                }
                Task::ShieldedTransfer(source, target, amount, setting) => {
                    if with_fee {
                        self.modify_balance_fee(&setting.gas_payer, setting.gas_limit);
                    }
                    self.modify_shielded_transfer(source, target, *amount);
                }
                Task::Shielding(source, target, amount, setting) => {
                    if with_fee {
                        self.modify_balance_fee(&setting.gas_payer, setting.gas_limit);
                    }
                    self.modify_shielding(source, target, *amount)
                }
                Task::Unshielding(source, target, amount, setting) => {
                    if with_fee {
                        self.modify_balance_fee(&setting.gas_payer, setting.gas_limit);
                    }
                    self.modify_unshielding(source, target, *amount)
                }
                Task::BecomeValidator(alias, _, _, _, _, _, _, setting) => {
                    if with_fee {
                        self.modify_balance_fee(&setting.gas_payer, setting.gas_limit);
                    }
                    self.set_enstablished_as_validator(alias)
                }
                Task::ChangeMetadata(_, _, _, _, _, _, setting) => {
                    if with_fee {
                        self.modify_balance_fee(&setting.gas_payer, setting.gas_limit);
                    }
                }
                Task::ChangeConsensusKey(_, _, setting) => {
                    if with_fee {
                        self.modify_balance_fee(&setting.gas_payer, setting.gas_limit);
                    }
                }
                Task::UpdateAccount(target, sources, threshold, setting) => {
                    if with_fee {
                        self.modify_balance_fee(&setting.gas_payer, setting.gas_limit);
                    }
                    self.modify_enstablished_account(target, sources, *threshold);
                }
                Task::DeactivateValidator(target, setting) => {
                    if with_fee {
                        self.modify_balance_fee(&setting.gas_payer, setting.gas_limit);
                    }
                    self.set_validator_as_deactivated(target);
                }
                Task::ReactivateValidator(target, setting) => {
                    if with_fee {
                        self.modify_balance_fee(&setting.gas_payer, setting.gas_limit);
                    }
                    self.remove_deactivate_validator(target);
                }
                Task::DefaultProposal(source, start_epoch, end_epoch, _, setting) => {
                    if with_fee {
                        self.modify_balance_fee(&setting.gas_payer, setting.gas_limit);
                    }
                    self.decrease_balance(source, PROPOSAL_DEPOSIT);
                    self.add_proposal(*start_epoch, *end_epoch);
                }
                Task::Vote(_, _, _, setting) => {
                    if with_fee {
                        self.modify_balance_fee(&setting.gas_payer, setting.gas_limit);
                    }
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
            if let Some(settings) = task.task_settings() {
                self.modify_balance_fee(&settings.gas_payer, settings.gas_limit);
            }
        }
    }

    /// File

    pub fn state_file_path(id: u64) -> PathBuf {
        env::current_dir()
            .expect("current directory")
            .join(format!("state-{id}.json"))
    }

    pub fn save(&self, locked_file: Option<fs::File>) -> Result<(), StateError> {
        let path = Self::state_file_path(self.id);
        let state_json = serde_json::to_string_pretty(&self).map_err(StateError::Serde)?;
        fs::write(path, state_json).map_err(StateError::File)?;

        if let Some(file) = locked_file {
            file.unlock().map_err(StateError::File)?;
        }

        Ok(())
    }

    pub fn create_new(id: u64, seed: Option<u64>) -> Result<(Self, fs::File), StateError> {
        // Lock the state file before writing the new
        let file = Self::lock_state_file(id)?;

        let state = Self::new(id, seed);
        state.save(None)?;

        Ok((state, file))
    }

    pub fn load(id: u64) -> Result<(Self, fs::File), StateError> {
        let path = Self::state_file_path(id);

        // Lock the state file before loading
        let file = Self::lock_state_file(id)?;

        let data = fs::read_to_string(path).map_err(StateError::File)?;
        if data.trim().is_empty() {
            return Err(StateError::EmptyFile);
        }
        let state = serde_json::from_str(&data).map_err(StateError::Serde)?;

        // Returns the file to be unlocked later
        Ok((state, file))
    }

    fn lock_state_file(id: u64) -> Result<fs::File, StateError> {
        let path = Self::state_file_path(id);

        let file = fs::File::open(&path).map_err(StateError::File)?;
        file.lock_exclusive().map_err(StateError::File)?;

        Ok(file)
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

    pub fn min_n_deactivated_validators(&self, sample: usize) -> bool {
        self.deactivated_validators.len() >= sample
    }

    pub fn any_votable_proposal(&self, current_epoch: u64) -> bool {
        self.proposals.iter().any(|(_, (start_epoch, end_epoch))| {
            current_epoch > *start_epoch && current_epoch < *end_epoch
        })
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

    pub fn random_deactivated_validator(
        &mut self,
        blacklist: Vec<Alias>,
        sample_size: usize,
    ) -> Vec<Account> {
        self.deactivated_validators
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

    pub fn random_account_with_min_balance(
        &mut self,
        blacklist: Vec<Alias>,
        min_balance: u64,
    ) -> Option<Account> {
        self.balances
            .iter()
            .filter_map(|(alias, balance)| {
                if blacklist.contains(alias) {
                    return None;
                }
                if balance >= &min_balance {
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

    pub fn random_votable_proposal(&mut self, current_epoch: u64) -> u64 {
        self.proposals
            .iter()
            .filter_map(|(proposal_id, (start_epoch, end_epoch))| {
                if current_epoch >= *start_epoch && current_epoch <= *end_epoch {
                    Some(proposal_id.to_owned())
                } else {
                    None
                }
            })
            .choose(&mut self.rng)
            .unwrap()
    }

    /// UPDATE

    pub fn add_implicit_account(&mut self, alias: &Alias) {
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

    pub fn add_masp_account(&mut self, alias: &Alias) {
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
        alias: &Alias,
        aliases: &BTreeSet<Alias>,
        threshold: u64,
    ) {
        self.accounts.insert(
            alias.clone(),
            Account {
                alias: alias.clone(),
                public_keys: aliases.clone(),
                threshold,
                address_type: AddressType::Enstablished,
            },
        );
        self.balances.insert(alias.clone(), 0);
    }

    pub fn modify_enstablished_account(
        &mut self,
        alias: &Alias,
        aliases: &BTreeSet<Alias>,
        threshold: u64,
    ) {
        self.accounts.entry(alias.clone()).and_modify(|account| {
            account.public_keys = aliases.clone();
            account.threshold = threshold;
        });
    }

    pub fn increase_balance(&mut self, target: &Alias, amount: u64) {
        if target.is_faucet() {
            return;
        }
        *self.balances.get_mut(target).unwrap() += amount;
    }

    pub fn decrease_balance(&mut self, target: &Alias, amount: u64) {
        if target.is_faucet() {
            return;
        }
        *self.balances.get_mut(target).unwrap() -= amount;
    }

    pub fn modify_balance_fee(&mut self, source: &Alias, _gas_limit: u64) {
        if !source.is_faucet() {
            *self.balances.get_mut(&source).unwrap() -= DEFAULT_FEE_IN_NATIVE_TOKEN;
        }
    }

    pub fn modify_bond(&mut self, source: &Alias, validator: &str, amount: u64) {
        if !source.is_faucet() {
            *self.balances.get_mut(&source).unwrap() -= amount;
        }
        let default = HashMap::from_iter([(validator.to_string(), 0u64)]);
        *self
            .bonds
            .entry(source.clone())
            .or_insert(default)
            .entry(validator.to_string())
            .or_insert(0) += amount;
    }

    pub fn modify_redelegate(&mut self, source: &Alias, from: &str, to: &str, amount: u64) {
        let default = HashMap::from_iter([(to.to_string(), 0u64)]);
        *self
            .redelegations
            .entry(source.clone())
            .or_insert(default)
            .entry(to.to_string())
            .or_insert(0) += amount;
        self.bonds
            .entry(source.clone())
            .and_modify(|bond| *bond.get_mut(from).unwrap() -= amount);
    }

    pub fn modify_unbonds(&mut self, source: &Alias, validator: &str, amount: u64) {
        let default = HashMap::from_iter([(validator.to_string(), 0u64)]);
        *self
            .unbonds
            .entry(source.clone())
            .or_insert(default)
            .entry(validator.to_string())
            .or_insert(0) += amount;
        self.bonds
            .entry(source.clone())
            .and_modify(|bond| *bond.get_mut(validator).unwrap() -= amount);
    }

    pub fn modify_shielding(&mut self, source: &Alias, target: &Alias, amount: u64) {
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

    pub fn modify_unshielding(&mut self, source: &Alias, target: &Alias, amount: u64) {
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

    pub fn modify_shielded_transfer(&mut self, source: &Alias, target: &Alias, amount: u64) {
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

    pub fn set_enstablished_as_validator(&mut self, alias: &Alias) {
        let account = self.accounts.remove(alias).unwrap();
        self.balances.remove(alias).unwrap();
        self.validators.insert(alias.clone(), account);
    }

    pub fn set_validator_as_deactivated(&mut self, alias: &Alias) {
        let account = self.validators.remove(alias).unwrap();
        self.deactivated_validators.insert(alias.clone(), account);
    }

    pub fn remove_deactivate_validator(&mut self, alias: &Alias) {
        self.deactivated_validators.remove(alias).unwrap();
    }

    pub fn add_proposal(&mut self, start_epoch: u64, end_epoch: u64) {
        let latest_proposal_id = self.proposals.keys().max().unwrap_or(&0).to_owned();
        self.proposals
            .insert(latest_proposal_id, (start_epoch, end_epoch));
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
