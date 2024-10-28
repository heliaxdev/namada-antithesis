use std::path::PathBuf;

use namada_sdk::{
    io::NullIo,
    masp::fs::FsShieldedUtils,
    wallet::{fs::FsWalletUtils, Wallet},
    NamadaImpl, ShieldedWallet,
};
use tendermint_rpc::HttpClient;

pub struct Sdk {
    pub base_dir: PathBuf,
    pub namada: NamadaImpl<HttpClient, FsWalletUtils, FsShieldedUtils, NullIo>,
    pub masp_indexer_url: String
}

impl Sdk {
    pub async fn new(
        base_dir: &PathBuf,
        http_client: HttpClient,
        wallet: Wallet<FsWalletUtils>,
        shielded_ctx: ShieldedWallet<FsShieldedUtils>,
        io: NullIo,
        masp_indexer_url: String
    ) -> Sdk {
        let namada = NamadaImpl::new(http_client, wallet, shielded_ctx, io)
            .await
            .expect("unable to construct Namada object");

        Self {
            base_dir: base_dir.to_owned(),
            namada,
            masp_indexer_url: format!("http://{}",masp_indexer_url)
        }
    }
}
