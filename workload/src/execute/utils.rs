use std::{path::PathBuf, str::FromStr};

use namada_sdk::{
    args::{self, DeviceTransport, TxBuilder},
    rpc::TxResponse,
    signing::{default_sign, SigningTxData},
    tx::{
        self,
        data::{GasLimit, TxType},
        either, ProcessTxResponse, Tx, TX_REVEAL_PK,
    },
    Namada,
};

use crate::{
    constants::DEFAULT_GAS_LIMIT, entities::Alias, sdk::namada::Sdk, steps::StepError,
    task::TaskSettings,
};

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

pub(crate) fn get_tx_errors(tx: &Tx, tx_response: &ProcessTxResponse) -> Option<String> {
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

async fn default_tx_arg(sdk: &Sdk) -> args::Tx {
    let wallet = sdk.namada.wallet.read().await;
    let nam = wallet
        .find_address("nam")
        .expect("Native token should be present.")
        .into_owned();

    args::Tx {
        dry_run: false,
        dry_run_wrapper: false,
        dump_tx: false,
        output_folder: None,
        force: false,
        broadcast_only: false,
        ledger_address: tendermint_rpc::Url::from_str("http://127.0.0.1:26657").unwrap(),
        initialized_account_alias: None,
        wallet_alias_force: false,
        fee_amount: None,
        wrapper_fee_payer: None,
        fee_token: nam,
        gas_limit: GasLimit::from(DEFAULT_GAS_LIMIT),
        expiration: Default::default(),
        chain_id: None,
        signing_keys: vec![],
        signatures: vec![],
        tx_reveal_code_path: PathBuf::from(TX_REVEAL_PK),
        password: None,
        memo: None,
        use_device: false,
        device_transport: DeviceTransport::default(),
        // dump_wrapper_tx: false,
        // wrapper_signature: None
    }
}

pub async fn merge_tx(
    sdk: &Sdk,
    txs: Vec<(Tx, SigningTxData)>,
    settings: TaskSettings,
) -> Result<(Tx, Vec<SigningTxData>, args::Tx), StepError> {
    if txs.is_empty() {
        return Err(StepError::Build("Empty tx batch".to_string()));
    }
    let tx_args = default_tx_arg(sdk).await;

    let wallet = sdk.namada.wallet.write().await;

    let faucet_alias = Alias::faucet();
    let gas_payer = wallet.find_public_key(faucet_alias.name).unwrap();
    drop(wallet);

    let (tx, signing_datas) = if txs.len() == 1 {
        let (tx, signing_data) = txs[0].clone();
        (tx, vec![signing_data])
    } else {
        let (mut tx, signing_datas) =
            tx::build_batch(txs.clone()).map_err(|e| StepError::Build(e.to_string()))?;
        tx.header.atomic = true;

        let mut wrapper = tx.header.wrapper().unwrap();
        wrapper.gas_limit = GasLimit::from(settings.gas_limit);
        wrapper.pk = gas_payer.clone();
        tx.header.tx_type = TxType::Wrapper(Box::new(wrapper));

        (tx, signing_datas)
    };

    tracing::info!("Built batch with {} txs.", txs.len());

    let tx_args = tx_args.wrapper_fee_payer(gas_payer);

    Ok((tx, signing_datas, tx_args))
}

pub(crate) async fn execute_tx(
    sdk: &Sdk,
    tx: &mut Tx,
    signing_datas: Vec<SigningTxData>,
    tx_args: &args::Tx,
) -> Result<Option<u64>, StepError> {
    do_sign_tx(sdk, tx, signing_datas, tx_args).await;

    let tx_response = sdk.namada.submit(tx.clone(), tx_args).await;

    if is_tx_rejected(tx, &tx_response) {
        match tx_response {
            Ok(tx_response) => {
                let errors = get_tx_errors(tx, &tx_response).unwrap_or_default();
                return Err(StepError::Execution(errors));
            }
            Err(e) => return Err(StepError::Broadcast(e.to_string())),
        }
    }

    if let Ok(ProcessTxResponse::Applied(TxResponse { height, .. })) = &tx_response {
        Ok(Some(height.0))
    } else {
        Ok(None)
    }
}

pub async fn do_sign_tx(
    sdk: &Sdk,
    tx: &mut Tx,
    signing_datas: Vec<SigningTxData>,
    tx_args: &args::Tx,
) {
    for signing_data in signing_datas {
        sdk.namada
            .sign(tx, tx_args, signing_data, default_sign, ())
            .await
            .expect("unable to sign tx");
    }
}
