use std::time::Duration;

use chrono::{DateTime, Timelike, Utc};
use tokio::time::sleep;

use crate::sdk::namada::Sdk;

pub mod epoch;
pub mod height;
pub mod inflation;
pub mod masp_indexer;
pub mod status;
pub mod voting_power;

const MAX_RETRY_COUNT: u64 = 8;
const RETRY_INTERVAL_SEC: u64 = 5;

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
        while times <= MAX_RETRY_COUNT {
            let result = Self::check(sdk, state).await;
            if result.is_ok() {
                return result;
            } else {
                if times == MAX_RETRY_COUNT {
                    tracing::error!(
                        "Check {} failed {} times, returning error",
                        Self::to_string(),
                        times
                    );
                    return result;
                }
                tracing::warn!(
                    "Check {} failed (error: {}) retrying ({}/{}),...",
                    Self::to_string(),
                    result.err().unwrap().to_string(),
                    times,
                    MAX_RETRY_COUNT,
                );
                times += 1;
                sleep(Duration::from_secs(RETRY_INTERVAL_SEC)).await
            }
        }
        Err(format!("Failed {} check (end)", Self::to_string()))
    }

    fn timing() -> u32;

    fn to_string() -> String;
}
