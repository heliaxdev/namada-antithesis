use std::path::PathBuf;

use serde::Deserialize;

use crate::error::Error;
use crate::step::StepType;

#[derive(clap::Parser, Clone, Debug)]
pub struct Args {
    #[clap(long, env)]
    #[arg(required = true)]
    pub config: PathBuf,
    #[arg(required = true)]
    pub step_type: StepType,
    #[clap(long, env)]
    #[clap(default_value_t = false)]
    pub no_check: bool,
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub id: u64,
    pub chain_id: String,
    pub rpc: String,
    pub masp_indexer_url: String,
    pub faucet_sk: String,
}

impl AppConfig {
    pub fn load(path: PathBuf) -> Result<Self, Error> {
        let content = std::fs::read_to_string(path).map_err(|e| Error::Config(e.to_string()))?;
        toml::from_str(&content).map_err(|e| Error::Config(e.to_string()))
    }
}
