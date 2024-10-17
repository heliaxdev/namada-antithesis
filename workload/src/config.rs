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
    #[arg(required = true)]
    pub step: String,
}
