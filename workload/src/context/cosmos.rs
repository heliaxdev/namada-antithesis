use std::path::Path;
use std::str::FromStr;

use bip32::{DerivationPath, Language, Mnemonic};
use cosmrs::crypto::secp256k1::SigningKey;
use cosmrs::rpc::HttpClient;
use cosmrs::AccountId;
use serde::Deserialize;

use crate::config::AppConfig;

pub struct CosmosCtx {
    pub client: HttpClient,
    pub account: AccountId,
    pub signing_key: SigningKey,
}

impl CosmosCtx {
    pub fn new(config: &AppConfig) -> Result<Self, String> {
        let client = HttpClient::new(&*config.cosmos_rpc).expect("invalid RPC");
        let wallet_path = config
            .cosmos_base_dir
            .join(format!("user_{}_seed.json", config.id));
        let (account, signing_key) = load_key(&wallet_path)?;
        Ok(Self {
            client,
            account,
            signing_key,
        })
    }
}

#[derive(Deserialize)]
struct WalletData {
    mnemonic: String,
    address: String,
}

const HD_PATH: &str = "m/44'/118'/0'/0/0";

fn load_key(path: &Path) -> Result<(AccountId, SigningKey), String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let wallet_data: WalletData = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    let account = AccountId::from_str(&wallet_data.address).map_err(|e| e.to_string())?;
    let mnemonic =
        Mnemonic::new(wallet_data.mnemonic, Language::English).map_err(|e| e.to_string())?;
    let seed = mnemonic.to_seed("");
    let signing_key =
        SigningKey::derive_from_path(&seed, &HD_PATH.parse::<DerivationPath>().unwrap())
            .map_err(|e| e.to_string())?;

    Ok((account, signing_key))
}
