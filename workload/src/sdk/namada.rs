use std::path::PathBuf;

use namada_sdk::{
    io::NullIo,
    masp::{fs::FsShieldedUtils, ShieldedContext},
    wallet::{fs::FsWalletUtils, Wallet},
    NamadaImpl,
};
use tendermint_rpc::HttpClient;

pub struct Sdk {
    pub base_dir: PathBuf,
    pub namada: NamadaImpl<HttpClient, FsWalletUtils, FsShieldedUtils, NullIo>,
}

impl Sdk {
    pub async fn new(
        base_dir: &PathBuf,
        http_client: HttpClient,
        wallet: Wallet<FsWalletUtils>,
        shielded_ctx: ShieldedContext<FsShieldedUtils>,
        io: NullIo,
    ) -> Sdk {
        let namada = NamadaImpl::new(http_client, wallet, shielded_ctx, io)
            .await
            .expect("unable to construct Namada object");

        Self {
            base_dir: base_dir.to_owned(),
            namada,
        }
    }
}
