use crate::{state::State, task::Task};

use super::utils;

pub async fn build_new_wallet_keypair(state: &mut State) -> Vec<Task> {
    let alias = utils::random_alias(state);
    vec![Task::NewWalletKeyPair(alias)]
}
