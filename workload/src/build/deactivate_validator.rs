use crate::{
    entities::Alias,
    state::State,
    steps::StepError,
    task::{Task, TaskSettings},
};

pub async fn build_deactivate_validator(state: &mut State) -> Result<Vec<Task>, StepError> {
    let account = state.random_validator(vec![], 1).pop().unwrap();

    let task_settings = TaskSettings::new(account.public_keys.clone(), Alias::faucet());

    Ok(vec![Task::DeactivateValidator(
        account.alias,
        task_settings,
    )])
}
