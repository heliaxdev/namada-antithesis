use rand::Rng;

use crate::state::State;

pub fn get_random_between(state: &mut State, min: u64, max: u64) -> u64 {
    state.rng.gen_range(min..max)
}
