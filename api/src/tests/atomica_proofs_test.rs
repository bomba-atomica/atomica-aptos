// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The Atomica proof endpoints, checked the way an external verifier checks them: every byte is
//! decoded and verified with production aptos-types code, starting from the genesis waypoint.

use super::new_test_context;
use crate::{atomica_proofs::AtomicaProofsApi, Context};
use aptos_api_test_context::{current_function_name, TestContext};
use aptos_crypto::hash::{CryptoHash, HashValue};
use aptos_storage_interface::{AptosDbError, DbReader};
use aptos_types::{
    contract_event::ContractEvent,
    epoch_change::{EpochChangeProof, Verifier},
    epoch_state::EpochState,
    ledger_info::LedgerInfoWithSignatures,
    proof::{
        definition::EventAccumulatorProof, TransactionAccumulatorProof, TransactionInfoWithProof,
    },
    transaction::{TransactionInfo, TransactionWithProof, Version},
    waypoint::Waypoint,
};
use poem::IntoResponse;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{str::FromStr, sync::Arc};

fn decode<T: DeserializeOwned>(value: &Value) -> T {
    let text = value.as_str().unwrap();
    let bytes = hex::decode(text.trim_start_matches("0x")).unwrap();
    bcs::from_bytes(&bytes).unwrap()
}

/// Fetches the genesis waypoint and the epoch change chain, verifies the chain from the waypoint,
/// and returns the trusted state of the current epoch.
async fn trusted_epoch_state(context: &TestContext) -> EpochState {
    let resp = context.get("/atomica/genesis_waypoint").await;
    let waypoint = Waypoint::from_str(resp["waypoint"].as_str().unwrap()).unwrap();
    assert_eq!(
        resp["version"].as_str().unwrap(),
        waypoint.version().to_string()
    );
    assert_eq!(
        hex::decode(resp["value"].as_str().unwrap().trim_start_matches("0x")).unwrap(),
        waypoint.value().to_vec()
    );
    let genesis: LedgerInfoWithSignatures = decode(&resp["ledger_info_with_signatures"]);
    waypoint.verify(genesis.ledger_info()).unwrap();

    let resp = context
        .get("/atomica/epoch_change_proof?start_epoch=0")
        .await;
    let ledger_infos: Vec<LedgerInfoWithSignatures> = resp["ledger_infos_with_signatures"]
        .as_array()
        .unwrap()
        .iter()
        .map(decode)
        .collect();
    assert!(!resp["more"].as_bool().unwrap());
    assert_eq!(ledger_infos[0], genesis);
    let proof = EpochChangeProof::new(ledger_infos, false);
    let last = proof.verify(&waypoint).unwrap();
    last.ledger_info().next_epoch_state().cloned().unwrap()
}

/// Verifies one event proof response with aptos-types and returns the verified event.
fn verify_event_proof(
    resp: &Value,
    epoch_state: &EpochState,
    version: Version,
    index: u64,
) -> ContractEvent {
    let signed: LedgerInfoWithSignatures = decode(&resp["ledger_info_with_signatures"]);
    // Quorum signature of the epoch's validator set (the check also pins the epoch).
    epoch_state.verify(&signed).unwrap();
    assert_eq!(resp["version"].as_str().unwrap(), version.to_string());
    assert_eq!(resp["event_index"].as_str().unwrap(), index.to_string());

    let txn_info: TransactionInfo = decode(&resp["transaction_info"]);
    let txn_proof: TransactionAccumulatorProof = decode(&resp["transaction_accumulator_proof"]);
    TransactionInfoWithProof::new(txn_proof, txn_info.clone())
        .verify(signed.ledger_info(), version)
        .unwrap();

    let event: ContractEvent = decode(&resp["event"]);
    let event_proof: EventAccumulatorProof = decode(&resp["event_accumulator_proof"]);
    event_proof
        .verify(txn_info.event_root_hash(), event.hash(), index)
        .unwrap();
    // The same proof does not verify another event.
    assert!(event_proof
        .verify(txn_info.event_root_hash(), HashValue::zero(), index)
        .is_err());
    event
}

/// The latest version whose transaction emitted at least one event.
fn latest_version_with_events(context: &TestContext) -> Version {
    let latest = context.context.db.get_latest_ledger_info_version().unwrap();
    (0..=latest)
        .rev()
        .find(|v| !stored_events(context, *v).is_empty())
        .unwrap()
}

fn stored_events(context: &TestContext, version: Version) -> Vec<ContractEvent> {
    let ledger_version = context.context.db.get_latest_ledger_info_version().unwrap();
    context
        .context
        .db
        .get_transaction_by_version(version, ledger_version, true)
        .unwrap()
        .events
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_atomica_proofs_event_proof_verifies_against_signed_ledger_info() {
    let mut context = new_test_context(current_function_name!());
    context.create_account().await;
    context.create_account().await;
    let epoch_state = trusted_epoch_state(&context).await;
    let latest = context.context.db.get_latest_ledger_info_version().unwrap();
    assert!(latest > 0);

    let mut verified = 0;
    for version in 0..=latest {
        let events = stored_events(&context, version);
        if events.is_empty() {
            continue;
        }
        // Genesis emits many events: check the first few and the last one of every transaction.
        let mut indexes: Vec<usize> = (0..events.len().min(3)).collect();
        indexes.push(events.len() - 1);
        indexes.dedup();
        for index in indexes {
            let resp = context
                .get(&format!("/atomica/event_proof/{}/{}", version, index))
                .await;
            let event = verify_event_proof(&resp, &epoch_state, version, index as u64);
            assert_eq!(event, events[index]);
            verified += 1;
        }
    }
    assert!(verified > 4, "only {} event proofs were checked", verified);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_atomica_proofs_epoch_change_proof_verifies_from_genesis_waypoint() {
    let context = new_test_context(current_function_name!());
    let epoch_state = trusted_epoch_state(&context).await;
    assert_eq!(epoch_state.epoch, 1);

    // An explicit range gives the same chain.
    let resp = context
        .get("/atomica/epoch_change_proof?start_epoch=0&end_epoch=1")
        .await;
    assert_eq!(
        resp["ledger_infos_with_signatures"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    // An empty, reversed or still open range is a bad request.
    for query in [
        "start_epoch=1&end_epoch=1",
        "start_epoch=1&end_epoch=0",
        "start_epoch=0&end_epoch=5",
        "start_epoch=7",
    ] {
        context
            .expect_status_code(400)
            .get(&format!("/atomica/epoch_change_proof?{}", query))
            .await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_atomica_proofs_unknown_version_and_bad_index() {
    let mut context = new_test_context(current_function_name!());
    context.create_account().await;
    let latest = context.context.db.get_latest_ledger_info_version().unwrap();

    let resp = context
        .expect_status_code(404)
        .get(&format!("/atomica/event_proof/{}/0", latest + 1000))
        .await;
    assert_eq!(resp["error_code"], "version_not_found");

    let version = latest_version_with_events(&context);
    let count = stored_events(&context, version).len();
    context
        .get(&format!("/atomica/event_proof/{}/{}", version, count - 1))
        .await;
    let resp = context
        .expect_status_code(400)
        .get(&format!("/atomica/event_proof/{}/{}", version, count))
        .await;
    assert_eq!(resp["error_code"], "invalid_input");
    context
        .expect_status_code(400)
        .get(&format!("/atomica/event_proof/{}/{}", version, u64::MAX))
        .await;
}

/// A DbReader whose ledger pruner has removed every version below `min_readable`.
struct PrunedBelow {
    inner: Arc<dyn DbReader>,
    min_readable: Version,
}

impl DbReader for PrunedBelow {
    fn get_read_delegatee(&self) -> &dyn DbReader {
        &*self.inner
    }

    fn get_transaction_by_version(
        &self,
        version: Version,
        ledger_version: Version,
        fetch_events: bool,
    ) -> aptos_storage_interface::Result<TransactionWithProof> {
        if version < self.min_readable {
            return Err(AptosDbError::LedgerPruned {
                data_type: "Transaction".to_string(),
                version,
                min_available_version: self.min_readable,
            });
        }
        self.inner
            .get_transaction_by_version(version, ledger_version, fetch_events)
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_atomica_proofs_pruned_version_is_not_found() {
    let mut context = new_test_context(current_function_name!());
    context.create_account().await;
    let served = latest_version_with_events(&context);
    assert!(served > 0);

    let pruned_db = Arc::new(PrunedBelow {
        inner: context.context.db.clone(),
        min_readable: served,
    });
    // The handler runs directly: a second poem server in this test binary would register the
    // API metrics twice (aptos-api is linked both as the crate under test and as a dependency of
    // the test context).
    let api = AtomicaProofsApi {
        context: Arc::new(Context::new(
            context.context.chain_id(),
            pruned_db,
            context.mempool.ac_client.clone(),
            context.context.node_config.as_ref().clone(),
            None,
        )),
    };
    let err = api.get_event_proof(0, 0).err().unwrap();
    let resp = err.into_response();
    assert_eq!(resp.status(), poem::http::StatusCode::NOT_FOUND);
    let body: Value = serde_json::from_slice(&resp.into_body().into_vec().await.unwrap()).unwrap();
    assert_eq!(body["error_code"], "version_pruned");

    // The version at the pruner's boundary is still served.
    assert!(api.get_event_proof(served, 0).is_ok());
}
