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

pub fn get_random_string(state: &mut State, length: usize) -> String {
    let mut result = String::new();
    for _ in 0..length {
        let c = state.rng.gen_range(0..62);
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
