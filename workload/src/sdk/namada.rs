use std::{path::PathBuf, str::FromStr};

use namada_sdk::io::NamadaIo;
use namada_sdk::{
    address::{Address, ImplicitAddress},
    args::TxBuilder,
    chain::ChainId,
    io::NullIo,
    key::common::SecretKey,
    masp::{fs::FsShieldedUtils, ShieldedContext},
    rpc, NamadaImpl,
};
use namada_wallet::{fs::FsWalletUtils, Wallet};
use tendermint_rpc::HttpClient;

use crate::config::AppConfig;

pub struct Sdk {
    pub base_dir: PathBuf,
    pub namada: NamadaImpl<HttpClient, FsWalletUtils, FsShieldedUtils, NullIo>,
    pub masp_indexer_url: String,
}

impl Sdk {
    pub async fn new(
        config: &AppConfig,
        base_dir: &PathBuf,
        http_client: HttpClient,
        wallet: Wallet<FsWalletUtils>,
        shielded_ctx: ShieldedContext<FsShieldedUtils>,
        io: NullIo,
    ) -> Result<Sdk, String> {
        let sk = SecretKey::from_str(&config.faucet_sk).unwrap();
        let public_key = sk.to_public();
        let address = Address::Implicit(ImplicitAddress::from(&public_key));

        let namada = NamadaImpl::new(http_client, wallet, shielded_ctx.into(), io)
            .await
            .map_err(|e| e.to_string())?;
        let namada = namada.chain_id(ChainId::from_str(&config.chain_id).unwrap());

        let mut namada_wallet = namada.wallet.write().await;
        namada_wallet
            .insert_keypair("faucet".to_string(), true, sk, None, Some(address), None)
            .unwrap();

        let native_token = rpc::query_native_token(namada.client())
            .await
            .map_err(|e| e.to_string())?;
        namada_wallet
            .insert_address("nam", native_token, true)
            .unwrap();
        drop(namada_wallet);

        Ok(Self {
            base_dir: base_dir.to_owned(),
            namada,
            masp_indexer_url: format!("{}/api/v1", config.masp_indexer_url.clone()),
        })
    }
}
