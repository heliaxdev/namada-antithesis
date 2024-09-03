use crate::entities::Alias;

pub type Target = Alias;
pub type PreBalance = namada_sdk::token::Amount;
pub type Amount = u64;

#[derive(Clone, Debug)]
pub enum Check {
    RevealPk(Target),
    BalanceTarget(Target, PreBalance, Amount),
    BalanceSource(Target, PreBalance, Amount),
}
