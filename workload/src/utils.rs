use std::path::PathBuf;
use std::time::Duration;

use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};

use crate::constants::{INIT_DELAY_SEC, MAX_DELAY_SEC, MAX_RETRY_COUNT};

mod query;
mod tx;

pub use query::*;
pub use tx::*;

pub fn base_dir() -> PathBuf {
    std::env::current_dir().unwrap().join("base")
}

pub type RetryConfig = RetryFutureConfig<ExponentialBackoff, NoOnRetry>;

pub fn retry_config() -> RetryConfig {
    RetryFutureConfig::new(MAX_RETRY_COUNT)
        .exponential_backoff(Duration::from_secs(INIT_DELAY_SEC))
        .max_delay(Duration::from_secs(MAX_DELAY_SEC))
}
