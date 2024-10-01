use std::time::Duration;

use chrono::{DateTime, Timelike, Utc};
use tokio::time::sleep;

use crate::sdk::namada::Sdk;

pub mod epoch;
pub mod height;
pub mod inflation;
pub mod masp;
pub mod status;
pub mod voting_power;

pub trait DoCheck {
    async fn check(sdk: &Sdk, state: &mut crate::state::State) -> Result<(), String>;

    async fn do_check(
        sdk: &Sdk,
        state: &mut crate::state::State,
        now: DateTime<Utc>,
    ) -> Result<(), String> {
        if now.second().rem_euclid(Self::timing()).ne(&0) {
            return Ok(());
        }

        let mut times = 0;
        while times <= 3 {
            let result = Self::check(sdk, state).await;
            if result.is_ok() {
                return result;
            } else {
                if times == 3 {
                    tracing::info!(
                        "Check {} failed {} times, returning error",
                        Self::to_string(),
                        times
                    );
                    return result;
                }
                tracing::info!(
                    "Check {} failed (error: {}) retrying ({}/{}),...",
                    Self::to_string(),
                    result.err().unwrap().to_string(),
                    times,
                    3
                );
                times += 1;
                sleep(Duration::from_secs(2)).await
            }
        }
        Err(format!("Failed {} check (end)", Self::to_string()))
    }

    fn timing() -> u32;

    fn to_string() -> String;
}
