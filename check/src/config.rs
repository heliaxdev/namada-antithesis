#[derive(clap::Parser, Clone, Debug)]
pub struct AppConfig {
    #[clap(long, env)]
    #[arg(required = true)]
    pub rpc: String,
    #[clap(long, env)]
    #[arg(required = true)]
    pub masp_indexer_url: String,
}
