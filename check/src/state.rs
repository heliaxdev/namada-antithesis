use namada_sdk::token;

#[derive(Clone, Debug, Default)]
pub struct State {
    pub last_block_height: u64,
    pub last_block_height_masp_indexer: u64,
    pub last_epoch: u64,
    pub last_total_supply: token::Amount,
    pub two_nodes_have_two_third: bool,
    pub last_proposal_id: Option<u64>,
    pub on_going_proposals: Vec<u64>,
}

impl State {
    pub fn from_height(height: u64) -> Self {
        Self {
            last_block_height: height,
            last_block_height_masp_indexer: 0,
            last_epoch: 0,
            last_total_supply: token::Amount::default(),
            two_nodes_have_two_third: true,
            last_proposal_id: None,
            on_going_proposals: Default::default(),
        }
    }
}
