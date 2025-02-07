use rand::{
    distributions::{uniform::SampleUniform, Alphanumeric, DistString},
    Rng,
};

use crate::{entities::Alias, state::State};

pub(crate) fn random_between<T: SampleUniform + std::cmp::PartialOrd>(
    state: &mut State,
    from: T,
    to: T,
) -> T {
    if from == to {
        from
    } else {
        state.rng.gen_range(from..=to)
    }
}

pub(crate) fn random_alias(state: &mut State) -> Alias {
    format!(
        "workload-generator-{}",
        Alphanumeric.sample_string(&mut state.rng, 8)
    )
    .into()
}

pub(crate) fn random_alias_with_suffix(state: &mut State, suffix: String) -> Alias {
    format!("{}-{}", random_alias(state).name, suffix).into()
}
