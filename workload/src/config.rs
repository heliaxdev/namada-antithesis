#[derive(clap::Parser, Clone)]
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
}
