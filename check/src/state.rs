use namada_sdk::token;

#[derive(Clone, Debug, Default)]
pub struct State {
    pub last_block_height: u64,
    pub last_epoch: u64,
    pub last_total_supply: token::Amount,
}

impl State {
    pub fn from_height(height: u64) -> Self {
        Self {
            last_block_height: height,
            last_epoch: 0,
            last_total_supply: token::Amount::default(),
        }
    }
}
