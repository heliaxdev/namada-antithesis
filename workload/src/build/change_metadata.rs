use crate::{
    entities::Alias,
    state::State,
    steps::StepError,
    task::{Task, TaskSettings},
};

use super::utils;

pub async fn build_change_metadata(state: &mut State) -> Result<Vec<Task>, StepError> {
    let account = state.random_validator(vec![], 1).pop().unwrap();

    let website = utils::get_random_string(state, 15);
    let email = format!("{}@test.com", utils::get_random_string(state, 5));
    let discord = utils::get_random_string(state, 10);
    let description = utils::get_random_string(state, 30);
    let avatar = utils::get_random_string(state, 20);

    let task_settings = TaskSettings::new(account.public_keys.clone(), Alias::faucet());

    Ok(vec![Task::ChangeMetadata(
        account.alias,
        website,
        email,
        discord,
        description,
        avatar,
        task_settings,
    )])
}
