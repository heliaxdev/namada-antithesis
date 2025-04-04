use crate::config::AppConfig;

mod namada;

pub struct Ctx {
    pub namada: namada::NamadaCtx,
    pub masp_indexer_url: String,
}

impl Ctx {
    pub async fn new(config: &AppConfig) -> Result<Self, String> {
        Ok(Self {
            namada: namada::namada_ctx(config).await?,
            masp_indexer_url: format!("{}/api/v1", config.masp_indexer_url.clone()),
        })
    }
}
