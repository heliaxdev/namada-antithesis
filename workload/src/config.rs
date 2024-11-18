use crate::steps::StepType;

#[derive(clap::Parser, Clone, Debug)]
pub struct AppConfig {
    #[clap(long, env)]
    #[arg(required = true)]
    pub rpc: String,
    #[clap(long, env)]
    #[arg(required = true)]
    pub faucet_sk: String,
    #[clap(long, env)]
    #[arg(required = true)]
    pub chain_id: String,
    #[clap(long, env)]
    pub seed: Option<u64>,
    #[clap(long, env)]
    pub id: u64,
    #[clap(long, env)]
    #[arg(required = true)]
    pub masp_indexer_url: String,
    #[arg(required = true)]
    #[arg(value_enum)]
    pub step_type: StepType,
    #[clap(long, env)]
    #[clap(default_value_t = false)]
    pub no_check: bool,
}
