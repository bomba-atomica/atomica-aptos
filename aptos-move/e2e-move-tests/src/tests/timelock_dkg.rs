// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! `TimelockDKGResult` validator transaction and the reconfiguration hooks, against the real framework.
//!
//! What runs here: the handler's chain-state checks (feature switch, session in progress, epoch,
//! quorum), the Move side of the result (`finish_with_timelock_dkg_result`), and the epoch change with and
//! without a result. The cryptographic checks of the handler (domain-separated certificate, `V0` against the
//! verification keys, claimed `mpk`) are tested in the `dkg` crate, where real dealings exist.
//!
//! No swarm runs here: this is a fake executor, not a network of validators.

use crate::MoveHarness;
use aptos_crypto::HashValue;
use aptos_dkg::ibe::{encode_g2, master_public_key, MasterScalar};
use aptos_types::{
    account_config::CORE_CODE_ADDRESS,
    aggregate_signature::AggregateSignature,
    dkg::{
        chunky_dkg::CertifiedAggregatedChunkySubtranscript,
        timelock_dkg::CertifiedTimelockDKGOutput, DKGTranscriptMetadata,
    },
    on_chain_config::{
        ChunkyDKGConfigMoveStruct, ConfigurationResource, OnChainChunkyDKGConfig, OnChainConfig,
    },
    transaction::{ExecutionStatus, Transaction, TransactionOutput, TransactionStatus},
    validator_txn::ValidatorTransaction,
};
use move_core_types::{account_address::AccountAddress, value::MoveValue};

const EPOCH_INTERVAL: u64 = 7_200_000_000;

/// BCS of a disabled `ChunkyDKGConfig` (an `Any`).
fn off_config() -> Vec<u8> {
    let config: ChunkyDKGConfigMoveStruct = OnChainChunkyDKGConfig::default_disabled().into();
    bcs::to_bytes(&config).unwrap()
}

fn signer_arg() -> Vec<u8> {
    MoveValue::Signer(CORE_CODE_ADDRESS)
        .simple_serialize()
        .unwrap()
}

fn current_epoch(h: &MoveHarness) -> u64 {
    ConfigurationResource::fetch_config(&h.executor.get_state_view())
        .unwrap()
        .unwrap()
        .epoch()
}

fn is_ready(h: &mut MoveHarness) -> bool {
    let out = h.execute_view_function(
        str::parse("0x1::ibe_config::is_ready").unwrap(),
        vec![],
        vec![],
    );
    bcs::from_bytes(&out.values.unwrap()[0]).unwrap()
}

fn enable(h: &mut MoveHarness) {
    h.executor.exec(
        "timelock_dkg",
        "enable",
        vec![],
        vec![signer_arg(), bcs::to_bytes(&120u64).unwrap()],
    );
}

fn output(epoch: u64) -> CertifiedTimelockDKGOutput {
    CertifiedTimelockDKGOutput {
        certified_transcript: CertifiedAggregatedChunkySubtranscript {
            metadata: DKGTranscriptMetadata {
                epoch,
                author: AccountAddress::ZERO,
            },
            transcript_bytes: vec![],
            signature: AggregateSignature::empty(),
        },
        mpk: vec![0u8; 96],
        public_parameters_hash: HashValue::zero(),
    }
}

fn run_vtxn(h: &mut MoveHarness, epoch: u64) -> TransactionOutput {
    let txn =
        Transaction::ValidatorTransaction(ValidatorTransaction::TimelockDKGResult(output(epoch)));
    let mut outputs = h.executor.execute_transaction_block(vec![txn]).unwrap();
    let output = outputs.pop().unwrap();
    if !output.status().is_discarded() {
        h.executor.apply_write_set(output.write_set());
    }
    output
}

/// The abort code the handler reports for a discarded vtxn.
fn discard_code(output: &TransactionOutput) -> String {
    format!("{:?}", output.status())
}

#[test]
fn result_is_discarded_while_the_feature_is_off() {
    let mut h = MoveHarness::new();
    let epoch = current_epoch(&h);
    let out = run_vtxn(&mut h, epoch);
    assert!(out.status().is_discarded(), "{}", discard_code(&out));
    assert!(!is_ready(&mut h));
}

#[test]
fn result_is_discarded_without_a_session_or_with_an_uncertified_aggregate() {
    let mut h = MoveHarness::new();
    enable(&mut h);
    let epoch = current_epoch(&h);
    // Enabled, but no session started yet.
    assert!(run_vtxn(&mut h, epoch).status().is_discarded());

    // A session is in progress. An output of a wrong epoch, and one with no signatures, are discarded.
    h.executor.exec(
        "timelock_dkg",
        "start",
        vec![],
        vec![
            bcs::to_bytes(&epoch).unwrap(),
            off_config(),
            bcs::to_bytes(&Vec::<u8>::new()).unwrap(),
            bcs::to_bytes(&Vec::<u8>::new()).unwrap(),
        ],
    );
    assert!(run_vtxn(&mut h, epoch + 1).status().is_discarded());
    assert!(run_vtxn(&mut h, epoch).status().is_discarded());
    assert!(!is_ready(&mut h));
}

fn finish_with_result(h: &mut MoveHarness, epoch: u64) {
    // A 4 virtual player result: weights [2, 2], threshold 3. Keys are valid G2 encodings.
    let key = |k: u64| encode_g2(&master_public_key(&MasterScalar::from(k))).to_vec();
    let vks: Vec<Vec<u8>> = (1..=4).map(key).collect();
    let _ = epoch;
    h.executor.exec(
        "reconfiguration_with_dkg",
        "finish_with_timelock_dkg_result",
        vec![],
        vec![
            signer_arg(),
            bcs::to_bytes(&vec![1u8, 2, 3]).unwrap(),
            bcs::to_bytes(&key(9)).unwrap(),
            bcs::to_bytes(&3u64).unwrap(),
            bcs::to_bytes(&vec![2u64, 2]).unwrap(),
            bcs::to_bytes(&EPOCH_INTERVAL).unwrap(),
            bcs::to_bytes(&vec![5u8; 32]).unwrap(),
            bcs::to_bytes(&vec![6u8; 32]).unwrap(),
            bcs::to_bytes(&vks).unwrap(),
        ],
    );
}

fn start_session(h: &mut MoveHarness, epoch: u64) {
    h.executor.exec(
        "timelock_dkg",
        "start",
        vec![],
        vec![
            bcs::to_bytes(&epoch).unwrap(),
            off_config(),
            bcs::to_bytes(&Vec::<u8>::new()).unwrap(),
            bcs::to_bytes(&Vec::<u8>::new()).unwrap(),
        ],
    );
}

fn end_epoch(h: &mut MoveHarness) {
    h.executor.exec(
        "reconfiguration_with_dkg",
        "finish",
        vec![],
        vec![signer_arg()],
    );
}

#[test]
fn completed_session_writes_the_epoch_config_after_the_epoch_change() {
    let mut h = MoveHarness::new();
    enable(&mut h);
    let epoch = current_epoch(&h);
    start_session(&mut h, epoch);
    finish_with_result(&mut h, epoch);
    // Still the old epoch: nothing is ready yet.
    assert!(!is_ready(&mut h));
    end_epoch(&mut h);
    assert_eq!(current_epoch(&h), epoch + 1);
    assert!(is_ready(&mut h));
    let out = h.execute_view_function(
        str::parse("0x1::ibe_config::get_epoch_config").unwrap(),
        vec![],
        vec![],
    );
    let values = out.values.unwrap();
    let got_epoch: u64 = bcs::from_bytes(&values[0]).unwrap();
    let threshold: u64 = bcs::from_bytes(&values[1]).unwrap();
    let total: u64 = bcs::from_bytes(&values[2]).unwrap();
    assert_eq!((got_epoch, threshold, total), (epoch + 1, 3, 4));
}

#[test]
fn failed_session_does_not_block_the_epoch_change_and_leaves_no_key() {
    let mut h = MoveHarness::new();
    enable(&mut h);
    let epoch = current_epoch(&h);
    start_session(&mut h, epoch);
    // No result arrives. The epoch changes anyway.
    end_epoch(&mut h);
    assert_eq!(current_epoch(&h), epoch + 1);
    assert!(!is_ready(&mut h));

    // register_timelock aborts with E_IBE_NOT_READY (2).
    let user = h.new_account_at(AccountAddress::from_hex_literal("0xa11ce").unwrap());
    let status = h.run_entry_function(
        &user,
        str::parse("0x1::ibe_config::register_timelock").unwrap(),
        vec![],
        vec![bcs::to_bytes(&(u64::MAX / 2)).unwrap()],
    );
    match status {
        TransactionStatus::Keep(ExecutionStatus::MoveAbort { code, .. }) => assert_eq!(code, 2),
        other => panic!("expected E_IBE_NOT_READY, got {other:?}"),
    }
}
