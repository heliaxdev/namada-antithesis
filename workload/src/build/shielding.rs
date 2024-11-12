use std::collections::BTreeSet;

use crate::{
    entities::Alias,
    state::State,
    steps::StepError,
    task::{Task, TaskSettings},
};

use super::utils;

pub async fn build_shield_sync(state: &mut State) -> Vec<Task> {
    vec![Task::Shielding(alias)]
}
