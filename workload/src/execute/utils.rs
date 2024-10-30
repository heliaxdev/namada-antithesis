use namada_sdk::tx::{either, ProcessTxResponse, Tx};

pub(crate) fn is_tx_rejected(
    tx: &Tx,
    tx_response: &Result<ProcessTxResponse, namada_sdk::error::Error>,
) -> bool {
    let cmt = tx.first_commitments().unwrap().to_owned();
    let wrapper_hash = tx.wrapper_hash();
    match tx_response {
        Ok(tx_result) => tx_result
            .is_applied_and_valid(wrapper_hash.as_ref(), &cmt)
            .is_none(),
        Err(_) => true,
    }
}

pub(crate)  fn get_tx_errors(tx: &Tx, tx_response: &ProcessTxResponse) -> Option<String> {
    let cmt = tx.first_commitments().unwrap().to_owned();
    let wrapper_hash = tx.wrapper_hash();
    match tx_response {
        ProcessTxResponse::Applied(result) => match &result.batch {
            Some(batch) => {
                tracing::info!("batch result: {:#?}", batch);
                match batch.get_inner_tx_result(wrapper_hash.as_ref(), either::Right(&cmt)) {
                    Some(Ok(res)) => {
                        let errors = res.vps_result.errors.clone();
                        let _status_flag = res.vps_result.status_flags;
                        let _rejected_vps = res.vps_result.rejected_vps.clone();
                        Some(serde_json::to_string(&errors).unwrap())
                    }
                    Some(Err(e)) => Some(e.to_string()),
                    _ => None,
                }
            }
            None => None,
        },
        _ => None,
    }
}