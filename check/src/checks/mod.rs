use crate::sdk::namada::Sdk;

pub mod epoch;
pub mod height;
pub mod inflation;

pub trait DoCheck {
    async fn do_check(sdk: &Sdk, state: &mut crate::state::State) -> Result<(), String>;
    fn to_string() -> String;
}
