use antithesis_sdk::random::AntithesisRng;
use rand::distributions::uniform::SampleUniform;
use rand::distributions::{Alphanumeric, DistString};
use rand::Rng;

use crate::types::Alias;

pub(crate) fn coin_flip(p: f64) -> bool {
    AntithesisRng.gen_bool(p)
}

pub(crate) fn random_between<T: SampleUniform + std::cmp::PartialOrd>(from: T, to: T) -> T {
    if from == to {
        from
    } else {
        AntithesisRng.gen_range(from..=to)
    }
}

pub(crate) fn random_alias() -> Alias {
    format!(
        "workload-generator-{}",
        Alphanumeric.sample_string(&mut AntithesisRng, 8)
    )
    .into()
}

pub fn get_random_string(length: usize) -> String {
    let mut result = String::new();
    for _ in 0..length {
        let c = AntithesisRng.gen_range(0..62);
        let c = if c < 26 {
            (b'a' + c) as char
        } else if c < 52 {
            (b'A' + c - 26) as char
        } else {
            (b'0' + c - 52) as char
        };
        result.push(c);
    }
    result
}

#[macro_export]
macro_rules! assert_always_step {
    ($msg:literal, $details:expr) => {
        antithesis_sdk::assert_always!(true, $msg, &$details)
    };
}

#[macro_export]
macro_rules! assert_sometimes_step {
    ($msg:literal, $details:expr) => {
        antithesis_sdk::assert_sometimes!(true, $msg, &$details)
    };
}

#[macro_export]
macro_rules! assert_unrechable_step {
    ($msg:literal, $details:expr) => {
        antithesis_sdk::assert_unreachable!($msg, &$details)
    };
}
