use namada_sdk::key::SchemeType;
use namada_sdk::masp::find_valid_diversifier;
use namada_sdk::masp_primitives::zip32;
use namada_sdk::PaymentAddress;
use rand::rngs::OsRng;

use crate::code::{Code, CodeType};
use crate::context::Ctx;
use crate::error::StepError;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task};
use crate::utils::{get_block_height, retry_config};
use crate::{assert_always_step, assert_unreachable_step};

use super::utils;

#[derive(Clone, Debug, Default)]
pub struct NewWalletKeyPair;

impl StepContext for NewWalletKeyPair {
    fn name(&self) -> String {
        "new-wallet-keypair".to_string()
    }

    async fn is_valid(&self, _ctx: &Ctx, _state: &State) -> Result<bool, StepError> {
        Ok(true)
    }

    async fn build_task(&self, ctx: &Ctx, _state: &State) -> Result<Vec<Task>, StepError> {
        let alias = utils::random_alias();

        let height = get_block_height(ctx, retry_config()).await?;

        let mut wallet = ctx.namada.wallet.write().await;

        wallet
            .gen_store_secret_key(
                SchemeType::Ed25519,
                Some(alias.name.clone()),
                true,
                None,
                &mut OsRng,
            )
            .ok_or_else(|| {
                StepError::Wallet(format!("Failed to generate keypair for {}", alias.name))
            })?;

        let spending_key_alias = alias.spending_key().name;
        let (_alias, spending_key) = wallet
            .gen_store_spending_key(
                spending_key_alias.clone(),
                Some(height.into()),
                None,
                true,
                &mut OsRng,
            )
            .ok_or_else(|| {
                StepError::Wallet(format!(
                    "Failed to generate spending key for {}",
                    spending_key_alias
                ))
            })?;

        let viewing_key = zip32::ExtendedFullViewingKey::from(&spending_key.into())
            .fvk
            .vk;
        let (div, _g_d) = find_valid_diversifier(&mut OsRng);
        let masp_payment_addr: namada_sdk::masp_primitives::sapling::PaymentAddress = viewing_key
            .to_payment_address(div)
            .expect("Conversion to PaymentAddress shouldn't fail");
        let payment_addr = PaymentAddress::from(masp_payment_addr);

        let payment_address_alias = alias.payment_address().name;
        wallet
            .insert_payment_addr(payment_address_alias.clone(), payment_addr, true)
            .ok_or_else(|| {
                StepError::Wallet(format!(
                    "Failed to insert payment address for {}",
                    payment_address_alias
                ))
            })?;

        wallet
            .save()
            .map_err(|e| StepError::Wallet(format!("Failed to save the wallet: {e}")))?;
        drop(wallet);

        Ok(vec![Task::NewWalletKeyPair(
            task::new_wallet_keypair::NewWalletKeyPair::builder()
                .source(alias)
                .build(),
        )])
    }

    fn assert(&self, code: &Code) {
        match code.code_type() {
            CodeType::Success => assert_always_step!("Done NewWalletKeyPair", code),
            CodeType::Fatal => assert_unreachable_step!("Fatal NewWalletKeyPair", code),
            CodeType::Skip => assert_unreachable_step!("Skipped NewWalletKeyPair", code),
            CodeType::Failed => assert_unreachable_step!("Failed NewWalletKeyPair", code),
        }
    }
}
