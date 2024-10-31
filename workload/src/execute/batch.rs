use namada_sdk::{signing::SigningTxData, tx::Tx};

use crate::{sdk::namada::Sdk, steps::StepError, task::TaskSettings};

use super::utils::{self, execute_tx};

pub async fn execute_tx_batch(
    sdk: &Sdk,
    txs: Vec<(Tx, SigningTxData)>,
    settings: TaskSettings,
) -> Result<Option<u64>, StepError> {
    let (mut tx, signing_datas, tx_args) = utils::merge_tx(sdk, txs, settings)
        .await
        .map_err(|e| StepError::Build(e.to_string()))?;

    execute_tx(sdk, &mut tx, signing_datas, &tx_args).await
}
