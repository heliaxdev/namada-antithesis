pub const NATIVE_SCALE: u64 = namada_sdk::token::NATIVE_SCALE;
pub const DEFAULT_GAS_LIMIT: u64 = 250_000;
pub const DEFAULT_GAS_PRICE: f64 = 0.000001;
pub const DEFAULT_FEE_IN_NATIVE_TOKEN: u64 = ((DEFAULT_GAS_LIMIT as f64 * DEFAULT_GAS_PRICE * NATIVE_SCALE as f64) + 10.0) as u64;
pub const MIN_TRANSFER_BALANCE: u64 = 2;