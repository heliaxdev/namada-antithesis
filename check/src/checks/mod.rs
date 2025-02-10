use std::time::Duration;

use chrono::{DateTime, Timelike, Utc};
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

pub async fn try_checks(sdk: &Sdk, state: &mut crate::state::State) {
    let now = chrono::offset::Utc::now();

    let vp_check_res = VotingPowerCheck::do_check(sdk, state, now).await;
    is_successful(VotingPowerCheck::to_string(), vp_check_res);

    let height_check_res = HeightCheck::do_check(sdk, state, now).await;
    is_successful(HeightCheck::to_string(), height_check_res);

    let epoch_check_res = EpochCheck::do_check(sdk, state, now).await;
    is_successful(EpochCheck::to_string(), epoch_check_res);

    let inflation_check_res = InflationCheck::do_check(sdk, state, now).await;
    is_successful(InflationCheck::to_string(), inflation_check_res);

    let status_check_res = StatusCheck::do_check(sdk, state, now).await;
    is_successful(StatusCheck::to_string(), status_check_res);

    let masp_indexer_check_res = MaspIndexerHeightCheck::do_check(sdk, state, now).await;
    is_successful(MaspIndexerHeightCheck::to_string(), masp_indexer_check_res);
}

trait DoCheck {
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

fn is_successful(check_name: String, res: Result<(), String>) {
    if let Err(e) = res.clone() {
        let is_timeout = e.to_lowercase().contains("timed out");
        let is_connection_closed = e.to_lowercase().contains("connection closed before");
        if is_timeout {
            tracing::warn!("Check {} has timedout", check_name);
            return;
        }
        if is_connection_closed {
            tracing::warn!(
                "Check {} has failed due to connection closed before message completed",
                check_name
            );
            return;
        }

        match check_name.as_ref() {
            "HeightCheck" => {
                antithesis_sdk::assert_always!(
                    res.is_ok(),
                    "Block height increased",
                    &json!({ "details": e })
                );
            }
            "EpochCheck" => {
                antithesis_sdk::assert_always!(
                    res.is_ok(),
                    "Epoch increased",
                    &json!({ "details": e })
                );
            }
            "InflationCheck" => {
                antithesis_sdk::assert_always!(
                    res.is_ok(),
                    "Inflation increased",
                    &json!({ "details": e })
                );
            }
            "MaspIndexerHeightCheck" => {
                antithesis_sdk::assert_sometimes!(
                    res.is_ok(),
                    "Masp indexer block height increased",
                    &json!({ "details": e })
                );
            }
            _ => {
                tracing::warn!("Check {} assertion not found (err)...", check_name);
            }
        }
        tracing::error!("{}", format!("Error! {}: {}", check_name, e));
    } else {
        match check_name.as_ref() {
            "HeightCheck" => {
                antithesis_sdk::assert_always!(res.is_ok(), "Block height increased", &json!({}));
            }
            "EpochCheck" => {
                antithesis_sdk::assert_always!(res.is_ok(), "Epoch increased", &json!({}));
            }
            "InflationCheck" => {
                antithesis_sdk::assert_always!(res.is_ok(), "Inflation increased", &json!({}));
            }
            "MaspIndexerHeightCheck" => {
                antithesis_sdk::assert_sometimes!(
                    res.is_ok(),
                    "Masp indexer block height increased",
                    &json!({})
                );
            }
            _ => {
                tracing::warn!("Check {} assertion not found...", check_name);
            }
        }
        tracing::debug!("{}", format!("Check {} was successful.", check_name));
    }
}
