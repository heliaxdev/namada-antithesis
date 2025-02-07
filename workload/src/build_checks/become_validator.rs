use crate::{check::Check, entities::Alias};

pub async fn become_validator(source: Alias) -> Vec<Check> {
    vec![Check::IsValidatorAccount(source)]
}
