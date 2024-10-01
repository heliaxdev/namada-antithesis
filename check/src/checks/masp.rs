use std::time::Duration;

use namada_sdk::{rpc, Namada};
use reqwest::{Request, StatusCode};

use crate::sdk::namada::Sdk;

use super::DoCheck;

#[derive(Clone, Debug, Default)]
pub struct MaspIndexerCheck {}

impl DoCheck for MaspIndexerCheck {
    async fn check(sdk: &Sdk, state: &mut crate::state::State) -> Result<(), String> {
        let client = reqwest::Client::builder().timeout(Duration::from_secs(5)).build().unwrap();
        let url = format!("http://{}/api/v1/health", sdk.masp_indexer_url);
        let res = client.get(url).send().await;
        match res {
            Ok(res) => {
                if res.status().is_success() {
                    Ok(())
                } else {
                    Err(format!("Can't connect to masp indexer webserver"))
                }
            },
            Err(e) => {
                Err(format!("Can't connect to masp indexer webserver: {}", e))
            },
        }
    }

    fn timing() -> u32 {
        30
    }

    fn to_string() -> String {
        "MaspIndexerCheck".to_string()
    }
}
