use std::time::Duration;

use chrono::{DateTime, Timelike, Utc};
use enum_dispatch::enum_dispatch;
use serde_json::json;
use tokio::time::sleep;

use crate::sdk::namada::Sdk;

pub mod epoch;
pub mod height;
pub mod inflation;
pub mod masp_indexer;
pub mod status;
pub mod voting_power;

use epoch::EpochCheck;
use height::HeightCheck;
use inflation::InflationCheck;
use masp_indexer::MaspIndexerHeightCheck;
use status::StatusCheck;
use voting_power::VotingPowerCheck;

const MAX_RETRY_COUNT: u64 = 8;
const RETRY_INTERVAL_SEC: u64 = 5;

#[enum_dispatch]
enum Checker {
    VotingPower(VotingPowerCheck),
    Height(HeightCheck),
    Epoch(EpochCheck),
    Inflation(InflationCheck),
    Status(StatusCheck),
    MaspIndexerHeight(MaspIndexerHeightCheck),
}

pub async fn try_checks(sdk: &Sdk, state: &mut crate::state::State) {
    let now = chrono::offset::Utc::now();

    let check_list = vec![
        Checker::VotingPower(VotingPowerCheck),
        Checker::Height(HeightCheck),
        Checker::Epoch(EpochCheck),
        Checker::Inflation(InflationCheck),
        Checker::Status(StatusCheck),
        Checker::MaspIndexerHeight(MaspIndexerHeightCheck),
    ];
    for checker in check_list {
        let vp_check_res = checker.do_check(sdk, state, now).await;
        is_successful(checker, vp_check_res);
    }
}

#[enum_dispatch(Checker)]
trait DoCheck {
    async fn check(&self, sdk: &Sdk, state: &mut crate::state::State) -> Result<(), String>;

    async fn do_check(
        &self,
        sdk: &Sdk,
        state: &mut crate::state::State,
        now: DateTime<Utc>,
    ) -> Result<(), String> {
        if now.second().rem_euclid(self.timing()).ne(&0) {
            return Ok(());
        }

        let mut times = 0;
        while times <= MAX_RETRY_COUNT {
            let result = self.check(sdk, state).await;
            if result.is_ok() {
                return result;
            } else {
                if times == MAX_RETRY_COUNT {
                    tracing::error!(
                        "Check {} failed {} times, returning error",
                        self.name(),
                        times
                    );
                    return result;
                }
                tracing::warn!(
                    "Check {} failed (error: {}) retrying ({}/{}),...",
                    self.name(),
                    result.err().unwrap().to_string(),
                    times,
                    MAX_RETRY_COUNT,
                );
                times += 1;
                sleep(Duration::from_secs(RETRY_INTERVAL_SEC)).await
            }
        }
        Err(format!("Failed {} check (end)", self.name()))
    }

    fn timing(&self) -> u32;

    fn name(&self) -> String;
}

fn is_successful(checker: Checker, res: Result<(), String>) {
    let details = if let Err(e) = res.clone() {
        let is_timeout = e.to_lowercase().contains("timed out");
        let is_connection_closed = e.to_lowercase().contains("connection closed before");
        if is_timeout {
            tracing::warn!("Check {} has timedout", checker.name());
            return;
        }
        if is_connection_closed {
            tracing::warn!(
                "Check {} has failed due to connection closed before message completed",
                checker.name()
            );
            return;
        }

        tracing::error!("{}", format!("Error! {}: {}", checker.name(), e));

        json!({ "details": e })
    } else {
        tracing::debug!("Check {} was successful.", checker.name());

        json!({})
    };
    match checker {
        Checker::VotingPower(_) => {
            antithesis_sdk::assert_always!(res.is_ok(), "Correct voting power", &details);
        }
        Checker::Height(_) => {
            antithesis_sdk::assert_always!(res.is_ok(), "Block height increased", &details);
        }
        Checker::Epoch(_) => {
            antithesis_sdk::assert_always!(res.is_ok(), "Epoch increased", &details);
        }
        Checker::Inflation(_) => {
            antithesis_sdk::assert_always!(res.is_ok(), "Inflation increased", &details);
        }
        Checker::Status(_) => {
            antithesis_sdk::assert_always!(res.is_ok(), "Healthy status", &details);
        }
        Checker::MaspIndexerHeight(_) => {
            antithesis_sdk::assert_sometimes!(
                res.is_ok(),
                "Masp indexer block height increased",
                &details
            );
        }
    }
}
