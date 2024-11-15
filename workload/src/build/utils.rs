use rand::{
    distributions::{Alphanumeric, DistString},
    Rng,
};

use crate::{entities::Alias, state::State};

pub(crate) fn random_between(state: &mut State, from: u64, to: u64) -> u64 {
    if from == to {
        return from;
    } else {
        state.rng.gen_range(from..to)
    }
}

pub(crate) fn random_alias(state: &mut State) -> Alias {
    format!(
        "workload-generator-{}",
        Alphanumeric.sample_string(&mut state.rng, 8)
    )
    .into()
}
