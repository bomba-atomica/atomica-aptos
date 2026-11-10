// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The Atomica proof endpoints on a live swarm: after a forced reconfiguration, the epoch change
//! chain and an event proof fetched over REST verify with aptos-types from the genesis waypoint.

use crate::smoke_test_environment::SwarmBuilder;
use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::hash::CryptoHash;
use aptos_forge::Swarm;
use aptos_types::{
    contract_event::ContractEvent,
    epoch_change::{EpochChangeProof, Verifier},
    epoch_state::EpochState,
    ledger_info::LedgerInfoWithSignatures,
    proof::{
        definition::EventAccumulatorProof, TransactionAccumulatorProof, TransactionInfoWithProof,
    },
    transaction::TransactionInfo,
    waypoint::Waypoint,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::str::FromStr;

fn decode<T: DeserializeOwned>(value: &Value) -> T {
    let text = value.as_str().unwrap();
    bcs::from_bytes(&hex::decode(text.trim_start_matches("0x")).unwrap()).unwrap()
}

async fn get(base: &str, path: &str) -> (reqwest::StatusCode, Value) {
    let resp = reqwest::get(format!("{}/{}", base.trim_end_matches('/'), path))
        .await
        .unwrap();
    let status = resp.status();
    (status, resp.json().await.unwrap())
}

/// Verifies the epoch change chain from the waypoint and returns the state of `epoch`.
async fn trusted_epoch_state(base: &str, waypoint: &Waypoint, epoch: u64) -> EpochState {
    let (status, resp) = get(
        base,
        &format!(
            "v1/atomica/epoch_change_proof?start_epoch=0&end_epoch={}",
            epoch
        ),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{}", resp);
    let ledger_infos: Vec<LedgerInfoWithSignatures> = resp["ledger_infos_with_signatures"]
        .as_array()
        .unwrap()
        .iter()
        .map(decode)
        .collect();
    assert_eq!(ledger_infos.len() as u64, epoch);
    let proof = EpochChangeProof::new(ledger_infos, resp["more"].as_bool().unwrap());
    let state = proof
        .verify(waypoint)
        .unwrap()
        .ledger_info()
        .next_epoch_state()
        .cloned()
        .unwrap();
    assert_eq!(state.epoch, epoch);
    state
}

#[tokio::test]
async fn test_outcome_proofs_across_epoch_change() {
    let swarm = SwarmBuilder::new_local(4).with_aptos().build().await;
    let config_waypoint = swarm
        .validators()
        .next()
        .unwrap()
        .config()
        .base
        .waypoint
        .genesis_waypoint();
    let mut info = swarm.aptos_public_info();
    let base = info.url().to_string();

    // The node serves the waypoint every validator was configured with.
    let (status, resp) = get(&base, "v1/atomica/genesis_waypoint").await;
    assert_eq!(status, reqwest::StatusCode::OK, "{}", resp);
    let waypoint = Waypoint::from_str(resp["waypoint"].as_str().unwrap()).unwrap();
    assert_eq!(waypoint, config_waypoint);

    // Force at least one reconfiguration, then emit events in the new epoch.
    let epoch_before = info
        .client()
        .get_ledger_information()
        .await
        .unwrap()
        .inner()
        .epoch;
    info.reconfig().await;
    let sender = info
        .create_and_fund_user_account(10_000_000_000)
        .await
        .unwrap();
    let receiver = info
        .create_and_fund_user_account(10_000_000_000)
        .await
        .unwrap();
    let txn = sender.sign_with_transaction_builder(
        info.transaction_factory()
            .payload(aptos_stdlib::aptos_coin_transfer(receiver.address(), 1)),
    );
    let committed = info
        .client()
        .submit_and_wait(&txn)
        .await
        .unwrap()
        .into_inner();
    let version = committed.version().unwrap();

    let mut verified = 0;
    for index in 0u64.. {
        let (status, resp) = get(
            &base,
            &format!("v1/atomica/event_proof/{}/{}", version, index),
        )
        .await;
        if status == reqwest::StatusCode::BAD_REQUEST {
            break;
        }
        assert_eq!(status, reqwest::StatusCode::OK, "{}", resp);

        let signed: LedgerInfoWithSignatures = decode(&resp["ledger_info_with_signatures"]);
        let epoch = signed.ledger_info().epoch();
        assert!(
            epoch > epoch_before,
            "no reconfiguration before epoch {}",
            epoch
        );
        let state = trusted_epoch_state(&base, &waypoint, epoch).await;
        state.verify(&signed).unwrap();

        let txn_info: TransactionInfo = decode(&resp["transaction_info"]);
        let txn_proof: TransactionAccumulatorProof = decode(&resp["transaction_accumulator_proof"]);
        TransactionInfoWithProof::new(txn_proof, txn_info.clone())
            .verify(signed.ledger_info(), version)
            .unwrap();
        assert!(txn_info.status().is_success());

        let event: ContractEvent = decode(&resp["event"]);
        let event_proof: EventAccumulatorProof = decode(&resp["event_accumulator_proof"]);
        event_proof
            .verify(txn_info.event_root_hash(), event.hash(), index)
            .unwrap();
        verified += 1;
    }
    assert!(
        verified > 0,
        "the transfer at version {} emitted no events",
        version
    );

    // A version past the ledger is not found.
    let (status, _) = get(&base, "v1/atomica/event_proof/18446744073709551615/0").await;
    assert_eq!(status, reqwest::StatusCode::NOT_FOUND);
}
