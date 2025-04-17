// Initialization
pub const INIT_IMPLICIT_ADDR_NUM: u64 = 10;
pub const INIT_ESTABLISHED_ADDR_NUM: u64 = 5;

// For retry
pub const MAX_RETRY_COUNT: u32 = 4;
pub const INIT_DELAY_SEC: u64 = 1;
pub const MAX_DELAY_SEC: u64 = 10;

// For batch
pub const MAX_BATCH_TX_NUM: u64 = 3;

// For bonding (They depend on the Namada parameters)
pub const PIPELINE_LEN: u64 = 2;
pub const UNBONDING_LEN: u64 = 3;

pub const NATIVE_SCALE: u64 = namada_sdk::token::NATIVE_SCALE;
pub const FAUCET_AMOUNT: u64 = 1_000_000 * NATIVE_SCALE;
pub const DEFAULT_GAS_PRICE: f64 = 0.000001;
pub const DEFAULT_GAS_LIMIT: u64 = namada_sdk::DEFAULT_GAS_LIMIT * 3;
pub const DEFAULT_FEE: u64 = DEFAULT_GAS_LIMIT * (DEFAULT_GAS_PRICE * NATIVE_SCALE as f64) as u64;
pub const MIN_TRANSFER_BALANCE: u64 = MAX_BATCH_TX_NUM * NATIVE_SCALE + DEFAULT_FEE;
pub const PROPOSAL_DEPOSIT: u64 = 50 * NATIVE_SCALE;

pub const COSMOS_CHAIN_ID: &str = "gaia-0";
pub const COSMOS_TOKEN: &str = "samoleans";
pub const COSMOS_FEE_TOKEN: &str = "stake";
pub const MAX_COSMOS_TRANSFER_AMOUNT: u64 = 100;
pub const COSMOS_FEE_AMOUNT: u64 = 50_000;
pub const COSMOS_GAS_LIMIT: u64 = 200_000;
