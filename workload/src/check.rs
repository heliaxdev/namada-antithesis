use std::collections::HashMap;
use std::fmt::{Display, Formatter};

use enum_dispatch::enum_dispatch;
use serde_json::json;

use crate::context::Ctx;
use crate::error::CheckError;
use crate::state::State;
use crate::types::{Alias, Balance, Fee, Height};
use crate::utils::{is_native_denom, RetryConfig};

pub mod account_exist;
pub mod balance_shielded_source;
pub mod balance_shielded_target;
pub mod balance_source;
pub mod balance_target;
pub mod bond_decrease;
pub mod bond_increase;
pub mod reveal_pk;
pub mod validator_account;
pub mod validator_status;
pub mod vote_result;

#[enum_dispatch]
pub enum Check {
    RevealPk(reveal_pk::RevealPk),
    BalanceTarget(balance_target::BalanceTarget),
    BalanceSource(balance_source::BalanceSource),
    BalanceShieldedTarget(balance_shielded_target::BalanceShieldedTarget),
    BalanceShieldedSource(balance_shielded_source::BalanceShieldedSource),
    BondIncrease(bond_increase::BondIncrease),
    BondDecrease(bond_decrease::BondDecrease),
    AccountExist(account_exist::AccountExist),
    IsValidatorAccount(validator_account::ValidatorAccount),
    ValidatorStatus(validator_status::ValidatorStatus),
    VoteResult(vote_result::VoteResult),
}

impl Display for Check {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

impl Check {
    pub fn assert_pre_balance(&self, state: &State) {
        let (matched, details) = match self {
            Check::BalanceSource(bs) => {
                let expected_pre_balance = if is_native_denom(bs.denom()) {
                    state.get_balance_for(bs.target())
                } else {
                    state.get_ibc_balance_for(bs.target(), bs.denom())
                };
                let matched = bs.pre_balance() == Balance::from_u64(expected_pre_balance);
                let details = json!({
                    "source_alias": bs.target(),
                    "denom": bs.denom(),
                    "expected_pre_balance": expected_pre_balance,
                    "actual_pre_balance": bs.pre_balance(),
                });
                antithesis_sdk::assert_always_or_unreachable!(
                    matched,
                    "Source pre balance matched",
                    &details
                );
                (matched, details)
            }
            Check::BalanceTarget(bt) => {
                let expected_pre_balance = if is_native_denom(bt.denom()) {
                    state.get_balance_for(bt.target())
                } else {
                    state.get_ibc_balance_for(bt.target(), bt.denom())
                };
                let matched = bt.pre_balance() == Balance::from_u64(expected_pre_balance);
                let details = json!({
                    "target_alias": bt.target(),
                    "denom": bt.denom(),
                    "expected_pre_balance": expected_pre_balance,
                    "actual_pre_balance": bt.pre_balance(),
                });
                antithesis_sdk::assert_always_or_unreachable!(
                    matched,
                    "Target pre balance matched",
                    &details
                );
                (matched, details)
            }
            Check::BalanceShieldedSource(bss) => {
                let expected_pre_balance = if is_native_denom(bss.denom()) {
                    state.get_shielded_balance_for(bss.target())
                } else {
                    state.get_ibc_balance_for(bss.target(), bss.denom())
                };
                let matched = bss.pre_balance() == Balance::from_u64(expected_pre_balance);
                let details = json!({
                    "source_alias": bss.target(),
                    "expected_pre_balance": expected_pre_balance,
                    "actual_pre_balance": bss.pre_balance(),
                });
                antithesis_sdk::assert_always_or_unreachable!(
                    matched,
                    "Source pre shielded balance matched",
                    &details
                );
                (matched, details)
            }
            Check::BalanceShieldedTarget(bst) => {
                let expected_pre_balance = if is_native_denom(bst.denom()) {
                    state.get_shielded_balance_for(bst.target())
                } else {
                    state.get_ibc_balance_for(bst.target(), bst.denom())
                };
                let matched = bst.pre_balance() == Balance::from_u64(expected_pre_balance);
                let details = json!({
                    "target_alias": bst.target(),
                    "expected_pre_balance": expected_pre_balance,
                    "actual_pre_balance": bst.pre_balance(),
                });
                antithesis_sdk::assert_always_or_unreachable!(
                    matched,
                    "Target pre shielded balance matched",
                    &details
                );
                (matched, details)
            }
            _ => (true, json!({})),
        };

        if !matched {
            tracing::error!("Pre-balance mismatched: {details}");
        }
    }
}

pub struct CheckInfo {
    pub execution_height: Height,
    pub check_height: Height,
}

#[enum_dispatch(Check)]
pub trait CheckContext {
    fn summary(&self) -> String;

    #[allow(async_fn_in_trait)]
    async fn do_check(
        &self,
        ctx: &Ctx,
        fees: &HashMap<Alias, Fee>,
        check_info: CheckInfo,
        retry_config: RetryConfig,
    ) -> Result<(), CheckError>;
}
