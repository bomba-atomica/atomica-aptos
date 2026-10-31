// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests of the timelock chunky DKG session (node-dkg-chunky-wiring).
//!
//! Two chunky DKG sessions run for the same dealers and the same target validator set in one epoch. One is
//! the mempool session (session id = `ChunkyDKGSessionMetadata`). The other is the timelock session
//! (`TimelockDKGSessionId`, timelock parameters, timelock certificate domain). These tests cover:
//!
//! - session separation in both directions: dealings, certificates, wire requests, manager routing,
//! - the checks of the `TimelockDKGResult` handler (`check_timelock_dkg_result`),
//! - `V0` against the interpolated verification keys,
//! - the node side: own shares against verification keys, the service state, the blob checks,
//! - that nothing of the mempool session changes (wire encoding, mempool manager behavior).
//!
//! No swarm runs here. The two-manager reconfiguration flow is covered by manager level tests with a
//! dummy network, not by a network of validators.

use crate::{
    chunky::{
        dkg_manager::ChunkyDKGManager,
        test_utils::{ChunkyTestSetup, DummyNetworkSender},
        types::{
            CertifiedAggregatedSubtranscript, ChunkyDKGSubtranscriptSignatureRequest,
            MissingTranscriptRequest,
        },
    },
    network::{DummyRpcResponseSender, IncomingRpcRequest},
    types::{
        ChunkyDKGTranscriptRequest, DKGMessage, TimelockDKGRequest, TimelockTranscriptRequest,
    },
};
use aptos_bitvec::BitVec;
use aptos_bounded_executor::BoundedExecutor;
use aptos_crypto::{HashValue, Uniform};
use aptos_dkg::{
    ibe::{
        compute_identity, decode_g1, decode_g2, derive_decryption_key, encode_g1, encode_g2,
        master_public_key, reconstruct_dk, verify_decryption_key, MasterScalar,
    },
    pvss::{
        traits::transcript::{Aggregatable, HasAggregatableSubtranscript},
        Player,
    },
};
use aptos_infallible::RwLock;
use aptos_network::{
    application::{interface::NetworkClient, storage::PeersAndMetadata},
    protocols::network::Event,
};
use aptos_reliable_broadcast::ReliableBroadcast;
use aptos_time_service::TimeService;
use aptos_types::{
    aggregate_signature::AggregateSignature,
    chain_id::ChainId,
    dkg::{
        chunky_dkg::{
            chunky_threshold_config, AggregatedSubtranscript,
            CertifiedAggregatedChunkySubtranscript, ChunkyDKGPublicParameters, ChunkyDKGStartEvent,
            ChunkyDecryptPrivKey, ChunkyTranscript, TimelockDKGSessionId,
        },
        timelock_dkg::{
            check_blob_hash_against_config, check_timelock_dkg_result, derive_own_shares,
            derive_timelock_config, derive_timelock_service_state, load_timelock_public_parameters,
            timelock_public_parameters_hash, validate_timelock_public_parameters_blob,
            CertifiedTimelockDKGOutput, ChunkyDKGSessionKind, ResultRejection,
            TimelockDKGSessionState, TimelockDKGState, TimelockEpochConfigResource,
            TimelockKeyError, TimelockPublicParameters, TimelockServiceRefusal,
            TimelockServiceState, TIMELOCK_PUBLIC_PARAMETERS,
        },
        DKGTranscriptMetadata,
    },
    validator_txn::{Topic, ValidatorTransaction},
};
use aptos_validator_transaction_pool::{TransactionFilter, VTxnPoolState};
use move_core_types::account_address::AccountAddress;
use once_cell::sync::Lazy;
use rand::{rngs::StdRng, thread_rng, SeedableRng};
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tokio::runtime::Handle;
use tokio_retry::strategy::ExponentialBackoff;

const EPOCH: u64 = 999;

/// Dealt once, shared by the tests: both sessions of one epoch, three dealers each.
struct Fixture {
    mempool: ChunkyTestSetup,
    timelock: ChunkyTestSetup,
    mempool_trxs: Vec<ChunkyTranscript>,
    timelock_trxs: Vec<ChunkyTranscript>,
    mempool_agg: AggregatedSubtranscript,
    timelock_agg: AggregatedSubtranscript,
}

fn aggregate(setup: &ChunkyTestSetup, trxs: &[ChunkyTranscript]) -> AggregatedSubtranscript {
    let agg = Aggregatable::aggregate(
        &setup.dkg_config.threshold_config,
        trxs.iter().map(|t| t.get_subtranscript()).collect(),
    )
    .unwrap();
    let mut dealer_bitmask = BitVec::with_num_bits(setup.addrs.len() as u16);
    for i in 0..trxs.len() {
        dealer_bitmask.set(i as u16);
    }
    AggregatedSubtranscript {
        dealer_epoch: EPOCH,
        subtranscript: agg,
        dealer_bitmask,
    }
}

static FIXTURE: Lazy<Fixture> = Lazy::new(|| {
    let mempool = ChunkyTestSetup::new_uniform(4);
    let timelock = mempool.timelock_twin();
    let dealers = [0usize, 1, 2];
    let mempool_trxs: Vec<_> = dealers
        .iter()
        .map(|&i| mempool.deal_transcript(i).1)
        .collect();
    let timelock_trxs: Vec<_> = dealers
        .iter()
        .map(|&i| timelock.deal_transcript(i).1)
        .collect();
    let mempool_agg = aggregate(&mempool, &mempool_trxs);
    let timelock_agg = aggregate(&timelock, &timelock_trxs);
    Fixture {
        mempool,
        timelock,
        mempool_trxs,
        timelock_trxs,
        mempool_agg,
        timelock_agg,
    }
});

fn sign(
    setup: &ChunkyTestSetup,
    kind: ChunkyDKGSessionKind,
    agg: &AggregatedSubtranscript,
    signers: &[usize],
) -> AggregateSignature {
    let sigs: BTreeMap<AccountAddress, _> = signers
        .iter()
        .map(|&i| {
            (
                setup.addrs[i],
                kind.sign_subtranscript(ChainId::test(), &setup.private_keys[i], agg)
                    .unwrap(),
            )
        })
        .collect();
    setup
        .epoch_state
        .verifier
        .aggregate_signatures(sigs.iter())
        .unwrap()
}

fn output_for(
    agg: &AggregatedSubtranscript,
    signature: AggregateSignature,
    mpk: Vec<u8>,
    hash: HashValue,
) -> CertifiedTimelockDKGOutput {
    CertifiedTimelockDKGOutput {
        certified_transcript: CertifiedAggregatedChunkySubtranscript {
            metadata: DKGTranscriptMetadata {
                epoch: EPOCH,
                author: AccountAddress::ZERO,
            },
            transcript_bytes: bcs::to_bytes(agg).unwrap(),
            signature,
        },
        mpk,
        public_parameters_hash: hash,
    }
}

fn local_hash() -> HashValue {
    TIMELOCK_PUBLIC_PARAMETERS.as_ref().unwrap().blob_hash
}

fn derived_mpk(f: &Fixture) -> Vec<u8> {
    let config = chunky_threshold_config(&f.timelock.session_metadata);
    derive_timelock_config(&f.timelock_agg.subtranscript, &config)
        .unwrap()
        .mpk
}

fn check(
    f: &Fixture,
    output: &CertifiedTimelockDKGOutput,
    epoch: u64,
    local: Option<HashValue>,
) -> Result<(), ResultRejection> {
    check_timelock_dkg_result(
        output,
        epoch,
        &f.timelock.session_metadata,
        ChainId::test(),
        &f.timelock.epoch_state.verifier,
        local,
    )
    .map(|_| ())
}

// ---- session separation of dealings ----

#[test]
fn two_sessions_one_reconfiguration_distinct_ids_unrelated_mpk() {
    let f = &*FIXTURE;
    // Same dealers, same metadata, different session ids.
    assert_eq!(f.mempool.session_metadata, f.timelock.session_metadata);
    assert_ne!(
        f.mempool.dkg_config.session_id_bytes(),
        f.timelock.dkg_config.session_id_bytes()
    );
    assert_eq!(
        f.timelock.dkg_config.session_id_bytes(),
        bcs::to_bytes(&TimelockDKGSessionId::new(
            ChainId::test(),
            f.timelock.session_metadata.clone()
        ))
        .unwrap()
    );
    // The mempool session id is the bare metadata, exactly as upstream.
    assert_eq!(
        f.mempool.dkg_config.session_id_bytes(),
        bcs::to_bytes(&f.mempool.session_metadata).unwrap()
    );

    let mut rng = thread_rng();
    let spks = f.mempool.spks();
    for t in &f.mempool_trxs {
        f.mempool
            .dkg_config
            .verify_transcript(t, &spks, &mut rng)
            .expect("mempool dealing under the mempool id");
        // Mempool dealing under the timelock id fails.
        assert!(f
            .timelock
            .dkg_config
            .verify_transcript(t, &spks, &mut rng)
            .is_err());
    }
    for t in &f.timelock_trxs {
        f.timelock
            .dkg_config
            .verify_transcript(t, &spks, &mut rng)
            .expect("timelock dealing under the timelock id");
        // Timelock dealing under the mempool id fails.
        assert!(f
            .mempool
            .dkg_config
            .verify_transcript(t, &spks, &mut rng)
            .is_err());
    }

    // Unrelated master public keys, and both interpolate (V0 against Vs).
    let mempool_cfg = chunky_threshold_config(&f.mempool.session_metadata);
    let m = derive_timelock_config(&f.mempool_agg.subtranscript, &mempool_cfg).unwrap();
    let t = derive_timelock_config(&f.timelock_agg.subtranscript, &mempool_cfg).unwrap();
    assert_ne!(m.mpk, t.mpk);
    assert_ne!(m.verification_keys, t.verification_keys);
}

// ---- session separation of certificates (replay defence) ----

#[test]
fn certificates_do_not_verify_in_the_other_domain() {
    let f = &*FIXTURE;
    let signers = [0usize, 1, 2];
    let verifier = &f.timelock.epoch_state.verifier;

    // Signature level, both directions, for the same aggregate bytes.
    let sk = &f.mempool.private_keys[0];
    let pk = &f.mempool.public_keys[0];
    let agg = &f.timelock_agg;
    let mempool_sig = ChunkyDKGSessionKind::Mempool
        .sign_subtranscript(ChainId::test(), sk, agg)
        .unwrap();
    let timelock_sig = ChunkyDKGSessionKind::Timelock
        .sign_subtranscript(ChainId::test(), sk, agg)
        .unwrap();
    assert_ne!(mempool_sig, timelock_sig);
    let ok = |kind: ChunkyDKGSessionKind, sig| {
        kind.verify_subtranscript_signature(ChainId::test(), agg, sig, pk)
            .is_ok()
    };
    assert!(ok(ChunkyDKGSessionKind::Mempool, &mempool_sig));
    assert!(ok(ChunkyDKGSessionKind::Timelock, &timelock_sig));
    assert!(!ok(ChunkyDKGSessionKind::Timelock, &mempool_sig));
    assert!(!ok(ChunkyDKGSessionKind::Mempool, &timelock_sig));
    // A different chain id is a different timelock domain.
    assert!(ChunkyDKGSessionKind::Timelock
        .verify_subtranscript_signature(ChainId::new(77), agg, &timelock_sig, pk)
        .is_err());

    // Aggregate level, both directions.
    let mempool_cert = sign(&f.mempool, ChunkyDKGSessionKind::Mempool, agg, &signers);
    let timelock_cert = sign(&f.timelock, ChunkyDKGSessionKind::Timelock, agg, &signers);
    let verify = |kind: ChunkyDKGSessionKind, cert: &AggregateSignature| {
        kind.verify_multi_signatures(ChainId::test(), verifier, agg, cert)
    };
    assert!(verify(ChunkyDKGSessionKind::Mempool, &mempool_cert).is_ok());
    assert!(verify(ChunkyDKGSessionKind::Timelock, &timelock_cert).is_ok());
    assert!(verify(ChunkyDKGSessionKind::Timelock, &mempool_cert).is_err());
    assert!(verify(ChunkyDKGSessionKind::Mempool, &timelock_cert).is_err());

    // At the check of the TimelockDKGResult handler: a certified mempool aggregate is refused,
    // even though it is signed by a quorum, and even for the timelock session's own aggregate bytes.
    let mpk = derived_mpk(f);
    let replayed = output_for(agg, mempool_cert, mpk.clone(), local_hash());
    assert_eq!(
        check(f, &replayed, EPOCH, Some(local_hash())),
        Err(ResultRejection::MultiSigVerificationFailed)
    );
    let genuine = output_for(agg, timelock_cert, mpk, local_hash());
    assert_eq!(check(f, &genuine, EPOCH, Some(local_hash())), Ok(()));
}

#[test]
fn certified_mempool_aggregate_never_becomes_a_timelock_result() {
    let f = &*FIXTURE;
    // The mempool session's own certified aggregate, submitted as a timelock result.
    let cert = sign(
        &f.mempool,
        ChunkyDKGSessionKind::Mempool,
        &f.mempool_agg,
        &[0, 1, 2],
    );
    let cfg = chunky_threshold_config(&f.mempool.session_metadata);
    let mpk = derive_timelock_config(&f.mempool_agg.subtranscript, &cfg)
        .unwrap()
        .mpk;
    let output = output_for(&f.mempool_agg, cert, mpk, local_hash());
    assert_eq!(
        check(f, &output, EPOCH, Some(local_hash())),
        Err(ResultRejection::MultiSigVerificationFailed)
    );
    // And the reverse: a timelock certificate does not verify as a mempool certificate.
    let timelock_cert = sign(
        &f.timelock,
        ChunkyDKGSessionKind::Timelock,
        &f.timelock_agg,
        &[0, 1, 2],
    );
    assert!(ChunkyDKGSessionKind::Mempool
        .verify_multi_signatures(
            ChainId::test(),
            &f.mempool.epoch_state.verifier,
            &f.timelock_agg,
            &timelock_cert
        )
        .is_err());
}

// ---- the checks of the TimelockDKGResult handler ----

#[test]
fn handler_check_rejects_each_fault() {
    let f = &*FIXTURE;
    let agg = &f.timelock_agg;
    let signers = [0usize, 1, 2];
    let cert = sign(&f.timelock, ChunkyDKGSessionKind::Timelock, agg, &signers);
    let mpk = derived_mpk(f);
    let good = output_for(agg, cert.clone(), mpk.clone(), local_hash());
    assert_eq!(check(f, &good, EPOCH, Some(local_hash())), Ok(()));
    // A node without the blob (a fullnode) trusts consensus for the hash only.
    assert_eq!(check(f, &good, EPOCH, None), Ok(()));

    // Epoch not current.
    assert_eq!(
        check(f, &good, EPOCH + 1, Some(local_hash())),
        Err(ResultRejection::EpochNotCurrent)
    );

    // Not enough voting power: one signer of four.
    let weak = sign(&f.timelock, ChunkyDKGSessionKind::Timelock, agg, &[0]);
    let weak_out = output_for(agg, weak, mpk.clone(), local_hash());
    assert_eq!(
        check(f, &weak_out, EPOCH, Some(local_hash())),
        Err(ResultRejection::NotEnoughVotingPower)
    );

    // Claimed mpk differs from the rederived one.
    let other_mpk = encode_g2(&master_public_key(&MasterScalar::from(5u64))).to_vec();
    let wrong_mpk = output_for(agg, cert.clone(), other_mpk, local_hash());
    assert_eq!(
        check(f, &wrong_mpk, EPOCH, Some(local_hash())),
        Err(ResultRejection::ClaimedMpkMismatch)
    );

    // Blob hash differs from this node's blob.
    let other_hash = HashValue::sha3_256_of(b"another blob");
    let wrong_hash = output_for(agg, cert.clone(), mpk.clone(), other_hash);
    assert_eq!(
        check(f, &wrong_hash, EPOCH, Some(local_hash())),
        Err(ResultRejection::ParametersHashMismatch)
    );

    // Undecodable subtranscript bytes.
    let mut garbled = good.clone();
    garbled.certified_transcript.transcript_bytes = vec![1, 2, 3];
    assert_eq!(
        check(f, &garbled, EPOCH, Some(local_hash())),
        Err(ResultRejection::TranscriptDeserializationFailed)
    );

    // Aggregate of another epoch.
    let mut stale = agg.clone();
    stale.dealer_epoch = EPOCH - 1;
    let stale_cert = sign(
        &f.timelock,
        ChunkyDKGSessionKind::Timelock,
        &stale,
        &signers,
    );
    let stale_out = output_for(&stale, stale_cert, mpk, local_hash());
    assert_eq!(
        check(f, &stale_out, EPOCH, Some(local_hash())),
        Err(ResultRejection::EpochNotCurrent)
    );
}

#[test]
fn mismatched_v0_is_rejected_even_when_certified() {
    let f = &*FIXTURE;
    // A quorum-signed subtranscript whose V0 does not belong to its Vs.
    let mut bad = f.timelock_agg.clone();
    let foreign = master_public_key(&MasterScalar::from(123456789u64));
    bad.subtranscript.V0 = foreign;
    let cert = sign(&f.timelock, ChunkyDKGSessionKind::Timelock, &bad, &[
        0, 1, 2,
    ]);
    // Even with the claimed mpk equal to the (wrong) V0, the interpolation check refuses it.
    let output = output_for(&bad, cert, encode_g2(&foreign).to_vec(), local_hash());
    match check(f, &output, EPOCH, Some(local_hash())) {
        Err(ResultRejection::KeyDerivationFailed(msg)) => {
            assert!(msg.contains("MpkMismatch"), "{msg}")
        },
        other => panic!("expected a V0 mismatch, got {other:?}"),
    }
    // A single corrupted verification key is caught too.
    let mut bad_vk = f.timelock_agg.clone();
    bad_vk.subtranscript.Vs[0][0] = foreign;
    let cert = sign(&f.timelock, ChunkyDKGSessionKind::Timelock, &bad_vk, &[
        0, 1, 2,
    ]);
    let output = output_for(&bad_vk, cert, derived_mpk(f), local_hash());
    assert!(matches!(
        check(f, &output, EPOCH, Some(local_hash())),
        Err(ResultRejection::KeyDerivationFailed(_))
    ));
}

// ---- node side: shares, service state ----

fn player_start(f: &Fixture, validator: usize) -> usize {
    let config = &f.timelock.dkg_config.threshold_config;
    config.get_player_starting_index(&Player { id: validator })
}

#[test]
fn own_shares_match_verification_keys_and_rebuild_the_key() {
    let f = &*FIXTURE;
    let params = TIMELOCK_PUBLIC_PARAMETERS.as_ref().unwrap();
    let config = &f.timelock.dkg_config.threshold_config;
    let derived = derive_timelock_config(&f.timelock_agg.subtranscript, config).unwrap();
    let mpk = decode_g2(&derived.mpk).unwrap();
    let identity = compute_identity(4, EPOCH + 1, 7, 1_000_000);

    // Every validator derives its shares from the subtranscript and its consensus key.
    let mut ids = vec![];
    let mut dk_shares = vec![];
    for v in 0..4 {
        let dk = ChunkyDecryptPrivKey::from(&*f.timelock.private_keys[v]);
        let own = derive_own_shares(
            &f.timelock_agg.subtranscript,
            config,
            &Player { id: v },
            &dk,
            &params.pp,
        )
        .unwrap_or_else(|e| panic!("validator {v}: {e}"));
        let start = player_start(f, v);
        assert_eq!(own.shares().len() as u64, derived.weights[v]);
        for (j, s) in own.shares().iter().enumerate() {
            // vk_j == g2 * share_j was checked by derive_own_shares. Check it here against the config.
            assert_eq!(
                encode_g2(&master_public_key(s)).to_vec(),
                derived.verification_keys[start + j]
            );
            ids.push(start + j);
            dk_shares.push(
                decode_g1(&encode_g1(&derive_decryption_key(s, &identity).unwrap())).unwrap(),
            );
        }
    }
    // The key rebuilt from the shares of all validators passes e(dk, g2) = e(H1(id), mpk). The master
    // secret is never reconstructed.
    let total = config.get_total_weight();
    let threshold = config.get_threshold_weight();
    let dk = reconstruct_dk(&ids, &dk_shares, threshold, total).unwrap();
    assert!(verify_decryption_key(&dk, &identity, &mpk));
    // A threshold subset gives the same key.
    let dk2 = reconstruct_dk(&ids[..threshold], &dk_shares[..threshold], threshold, total).unwrap();
    assert_eq!(dk, dk2);

    // A share of the other session fails the pairing check under this session's keys (review C-2).
    let mempool_cfg = &f.mempool.dkg_config.threshold_config;
    let m = derive_timelock_config(&f.mempool_agg.subtranscript, mempool_cfg).unwrap();
    let dk0 = ChunkyDecryptPrivKey::from(&*f.mempool.private_keys[0]);
    let mempool_shares = derive_own_shares(
        &f.mempool_agg.subtranscript,
        mempool_cfg,
        &Player { id: 0 },
        &dk0,
        &f.mempool.dkg_config.public_parameters,
    )
    .unwrap();
    let foreign = derive_decryption_key(&mempool_shares.shares()[0], &identity).unwrap();
    let vk = decode_g2(&derived.verification_keys[0]).unwrap();
    assert!(!aptos_dkg::ibe::verify_dk_share(&foreign, &identity, &vk));
    let own_vk = decode_g2(&m.verification_keys[0]).unwrap();
    assert!(aptos_dkg::ibe::verify_dk_share(
        &foreign, &identity, &own_vk
    ));
    assert_ne!(m.mpk, derived.mpk);
}

#[test]
fn share_that_does_not_match_its_key_stops_the_node() {
    let f = &*FIXTURE;
    let params = TIMELOCK_PUBLIC_PARAMETERS.as_ref().unwrap();
    let config = &f.timelock.dkg_config.threshold_config;
    // The on-chain subtranscript has a verification key that the validator's decrypted share does not match.
    let mut tampered = f.timelock_agg.subtranscript.clone();
    tampered.Vs[1][0] = master_public_key(&MasterScalar::from(99u64));
    let dk = ChunkyDecryptPrivKey::from(&*f.timelock.private_keys[1]);
    match derive_own_shares(&tampered, config, &Player { id: 1 }, &dk, &params.pp) {
        Err(TimelockKeyError::ShareMismatch { virtual_player }) => {
            assert_eq!(virtual_player, player_start(f, 1))
        },
        other => panic!("expected a share mismatch, got {:?}", other.map(|_| ())),
    }
    // A validator with the wrong decryption key cannot pass the check either.
    let wrong_dk = ChunkyDecryptPrivKey::from(&*f.timelock.private_keys[2]);
    assert!(derive_own_shares(
        &f.timelock_agg.subtranscript,
        config,
        &Player { id: 1 },
        &wrong_dk,
        &params.pp
    )
    .is_err());
}

fn epoch_config(f: &Fixture, params: &TimelockPublicParameters) -> TimelockEpochConfigResource {
    let config = &f.timelock.dkg_config.threshold_config;
    let derived = derive_timelock_config(&f.timelock_agg.subtranscript, config).unwrap();
    TimelockEpochConfigResource {
        epoch: EPOCH + 1,
        mpk: derived.mpk,
        reconstruct_threshold_weight: derived.reconstruct_threshold_weight,
        total_weight: config.get_total_weight() as u64,
        weights: derived.weights,
        epoch_interval_us: 7_200_000_000,
        public_parameters_hash: params.blob_hash.to_vec(),
        subtranscript_hash: aptos_crypto::hash::CryptoHash::hash(&f.timelock_agg).to_vec(),
        verification_keys: derived.verification_keys,
    }
}

fn chain_state(f: &Fixture) -> TimelockDKGState {
    TimelockDKGState {
        enabled: true,
        grace_period_secs: 120,
        last_completed: Some(TimelockDKGSessionState {
            metadata: f.timelock.session_metadata.clone(),
            start_time_us: 1,
            transcript: bcs::to_bytes(&f.timelock_agg).unwrap(),
        }),
        in_progress: None,
        pending_config: None,
    }
}

#[test]
fn service_state_is_ready_only_when_everything_matches() {
    let f = &*FIXTURE;
    let params = TIMELOCK_PUBLIC_PARAMETERS.as_ref().unwrap();
    let cfg = epoch_config(f, params);
    let state = chain_state(f);
    let dk = ChunkyDecryptPrivKey::from(&*f.timelock.private_keys[3]);
    let me = Player { id: 3 };
    let run = |cfg: Option<&TimelockEpochConfigResource>,
               state: &TimelockDKGState,
               params: Option<&TimelockPublicParameters>| {
        derive_timelock_service_state(EPOCH + 1, cfg, state, params, &me, &dk)
    };

    assert!(run(Some(&cfg), &state, Some(params)).is_ready());

    // No config for this epoch: the timelock DKG did not complete.
    assert!(matches!(
        run(None, &state, Some(params)),
        TimelockServiceState::Refused {
            reason: TimelockServiceRefusal::NoKeyThisEpoch,
            ..
        }
    ));
    let mut stale = cfg.clone();
    stale.epoch = EPOCH;
    assert!(matches!(
        run(Some(&stale), &state, Some(params)),
        TimelockServiceState::Refused {
            reason: TimelockServiceRefusal::NoKeyThisEpoch,
            ..
        }
    ));

    // No parameters on this node.
    assert!(matches!(
        run(Some(&cfg), &state, None),
        TimelockServiceState::Refused {
            reason: TimelockServiceRefusal::NoParameters,
            ..
        }
    ));

    // The blob hash differs from the on-chain hash (B-4): the node refuses to start the service.
    let mut bad_hash = cfg.clone();
    bad_hash.public_parameters_hash = vec![0u8; 32];
    assert!(matches!(
        run(Some(&bad_hash), &state, Some(params)),
        TimelockServiceState::Refused {
            reason: TimelockServiceRefusal::ParametersHashMismatch(_),
            ..
        }
    ));

    // The subtranscript on chain is not the one the config came from.
    let mut bad_sub = cfg.clone();
    bad_sub.subtranscript_hash = vec![9u8; 32];
    assert!(matches!(
        run(Some(&bad_sub), &state, Some(params)),
        TimelockServiceState::Refused {
            reason: TimelockServiceRefusal::SubtranscriptMismatch(_),
            ..
        }
    ));

    // On-chain verification keys that are not the subtranscript's.
    let mut bad_vk = cfg.clone();
    bad_vk.verification_keys[0] = bad_vk.verification_keys[1].clone();
    assert!(matches!(
        run(Some(&bad_vk), &state, Some(params)),
        TimelockServiceState::Refused {
            reason: TimelockServiceRefusal::VerificationKeyMismatch,
            ..
        }
    ));

    // A validator whose key does not decrypt the shares refuses.
    let wrong_dk = ChunkyDecryptPrivKey::from(&*f.timelock.private_keys[0]);
    assert!(matches!(
        derive_timelock_service_state(EPOCH + 1, Some(&cfg), &state, Some(params), &me, &wrong_dk),
        TimelockServiceState::Refused {
            reason: TimelockServiceRefusal::KeyDerivation(_),
            ..
        }
    ));
}

// ---- public parameters blob (B-1, B-3, B-4 analogues) ----

/// Small parameters with commitment base `g2^k`. `k = 1` is the standard generator.
fn small_params_blob(k: u64) -> Vec<u8> {
    let base = master_public_key(&MasterScalar::from(k));
    let mut rng = StdRng::seed_from_u64(31);
    let pp = ChunkyDKGPublicParameters::new_for_testing(8, 16, 4, base, &mut rng);
    bcs::to_bytes(&pp).unwrap()
}

#[test]
fn blob_loads_with_validation_and_refuses_a_corrupted_point() {
    let blob = small_params_blob(1);
    let loaded = load_timelock_public_parameters(&blob).expect("valid blob");
    assert_eq!(loaded.blob_hash, timelock_public_parameters_hash(&blob));

    // G_2 base that is not the standard generator is refused, although it decodes.
    let nonstandard = small_params_blob(2);
    assert!(bcs::from_bytes::<ChunkyDKGPublicParameters>(&nonstandard).is_ok());
    assert!(load_timelock_public_parameters(&nonstandard).is_err());

    // One corrupted point: the first ElGamal base G sits right after the length prefix. Find a change
    // that still decodes (the decoder skips the subgroup check) and that validation must refuse.
    let mut refused = 0;
    for delta in 1..=60u8 {
        let mut bad = blob.clone();
        bad[1 + 47] = bad[1 + 47].wrapping_add(delta);
        if bad == blob || bcs::from_bytes::<ChunkyDKGPublicParameters>(&bad).is_err() {
            continue;
        }
        let err =
            load_timelock_public_parameters(&bad).expect_err("corrupted blob must be refused");
        assert!(format!("{err:#}").contains("validation"), "{err:#}");
        refused += 1;
        if refused >= 3 {
            break;
        }
    }
    assert!(
        refused > 0,
        "no corrupted blob decoded: the test lost its subject"
    );
}

#[test]
fn validation_command_runs_the_real_validation() {
    let blob = small_params_blob(1);
    let report = validate_timelock_public_parameters_blob(&blob, None).unwrap();
    assert_eq!(report.blob_bytes, blob.len());
    assert_eq!(report.hash_matches_expected, None);

    // Parse and size succeed for this blob, and the command still says no. It is not a parse-only check.
    let nonstandard = small_params_blob(3);
    assert!(bcs::from_bytes::<ChunkyDKGPublicParameters>(&nonstandard).is_ok());
    assert!(validate_timelock_public_parameters_blob(&nonstandard, None).is_err());

    // The expected hash is compared (B-4).
    let hash = timelock_public_parameters_hash(&blob);
    let ok = validate_timelock_public_parameters_blob(&blob, Some(hash.as_ref())).unwrap();
    assert_eq!(ok.hash_matches_expected, Some(true));
    assert!(validate_timelock_public_parameters_blob(&blob, Some(&[0u8; 32])).is_err());
    let loaded = load_timelock_public_parameters(&blob).unwrap();
    assert!(check_blob_hash_against_config(&loaded, hash.as_ref()).is_ok());
    assert!(check_blob_hash_against_config(&loaded, &[1u8; 32]).is_err());
}

// ---- wire format and routing ----

#[test]
fn mempool_wire_encoding_is_unchanged_and_timelock_requests_are_appended() {
    let plain = DKGMessage::ChunkyTranscriptRequest(ChunkyDKGTranscriptRequest::new(7));
    let plain_bytes = bcs::to_bytes(&plain).unwrap();
    // Variant 2, then the epoch. Same as before the timelock session existed.
    assert_eq!(plain_bytes[0], 2);
    assert_eq!(&plain_bytes[1..], &7u64.to_le_bytes());

    let timelock: DKGMessage = TimelockTranscriptRequest(ChunkyDKGTranscriptRequest::new(7)).into();
    let timelock_bytes = bcs::to_bytes(&timelock).unwrap();
    // Variant 8 (appended), inner variant 0, then the epoch.
    assert_eq!(timelock_bytes[0], 8);
    assert_eq!(timelock_bytes[1], 0);
    assert_eq!(&timelock_bytes[2..], &7u64.to_le_bytes());
    assert_eq!(
        bcs::from_bytes::<DKGMessage>(&timelock_bytes).unwrap(),
        timelock
    );
    assert_eq!(timelock.epoch(), 7);

    assert_eq!(
        plain.clone().split_session_kind().0,
        ChunkyDKGSessionKind::Mempool
    );
    let (kind, back) = timelock.split_session_kind();
    assert_eq!(kind, ChunkyDKGSessionKind::Timelock);
    assert_eq!(back, plain);
    let _ = TimelockDKGRequest::MissingTranscript(MissingTranscriptRequest::new(
        1,
        AccountAddress::ONE,
    ));
}

fn manager(
    setup: &ChunkyTestSetup,
    kind: ChunkyDKGSessionKind,
    pool: VTxnPoolState,
) -> ChunkyDKGManager {
    let my_addr = setup.addrs[0];
    let rb = Arc::new(ReliableBroadcast::new(
        "test",
        my_addr,
        setup.addrs.clone(),
        Arc::new(DummyNetworkSender),
        ExponentialBackoff::from_millis(10),
        TimeService::real(),
        Duration::from_millis(500),
        BoundedExecutor::new(2, Handle::current()),
    ));
    let client: NetworkClient<DKGMessage> = NetworkClient::new(
        vec![],
        vec![],
        Default::default(),
        PeersAndMetadata::new(&[]),
    );
    let (self_sender, _rx): (
        aptos_channels::Sender<Event<DKGMessage>>,
        aptos_channels::Receiver<Event<DKGMessage>>,
    ) = aptos_channels::new_test(1);
    let network_sender = Arc::new(crate::network::NetworkSender::new(
        my_addr,
        crate::network_interface::DKGNetworkClient::new(client),
        self_sender,
    ));
    ChunkyDKGManager::new_with_kind(
        kind,
        ChainId::test(),
        setup.private_keys[0].clone(),
        Arc::new(setup.public_keys[0].clone()),
        0,
        my_addr,
        setup.epoch_state.clone(),
        pool,
        rb,
        network_sender,
    )
}

fn rpc(
    msg: DKGMessage,
    sender: AccountAddress,
    collector: Arc<RwLock<Vec<anyhow::Result<DKGMessage>>>>,
) -> IncomingRpcRequest {
    IncomingRpcRequest {
        msg,
        sender,
        response_sender: Box::new(DummyRpcResponseSender::new(collector)),
    }
}

#[tokio::test]
async fn a_manager_serves_only_its_own_session() {
    let f = &*FIXTURE;
    let mempool_mgr = &mut manager(
        &f.mempool,
        ChunkyDKGSessionKind::Mempool,
        VTxnPoolState::default(),
    );
    let timelock_mgr = &mut manager(
        &f.timelock,
        ChunkyDKGSessionKind::Timelock,
        VTxnPoolState::default(),
    );
    let plain = || DKGMessage::ChunkyTranscriptRequest(ChunkyDKGTranscriptRequest::new(EPOCH));
    let wrapped: fn() -> DKGMessage =
        || TimelockTranscriptRequest(ChunkyDKGTranscriptRequest::new(EPOCH)).into();

    // The timelock manager refuses a mempool request, the mempool manager refuses a timelock request.
    let c = Arc::new(RwLock::new(vec![]));
    assert!(timelock_mgr
        .process_peer_rpc_msg(rpc(plain(), f.timelock.addrs[3], c.clone()))
        .await
        .is_err());
    assert!(mempool_mgr
        .process_peer_rpc_msg(rpc(wrapped(), f.mempool.addrs[3], c.clone()))
        .await
        .is_err());

    // Each accepts its own kind (an error response in Init state is still a handled request).
    assert!(timelock_mgr
        .process_peer_rpc_msg(rpc(wrapped(), f.timelock.addrs[3], c.clone()))
        .await
        .is_ok());
    assert!(mempool_mgr
        .process_peer_rpc_msg(rpc(plain(), f.mempool.addrs[3], c.clone()))
        .await
        .is_ok());

    // The signature request and the missing transcript request are split the same way.
    let sig_req = ChunkyDKGSubtranscriptSignatureRequest::new(
        EPOCH,
        HashValue::zero(),
        BitVec::with_num_bits(4),
        vec![],
    );
    assert!(timelock_mgr
        .process_peer_rpc_msg(rpc(
            DKGMessage::SubtranscriptSignatureRequest(sig_req.clone()),
            f.timelock.addrs[3],
            c.clone()
        ))
        .await
        .is_err());
    assert!(mempool_mgr
        .process_peer_rpc_msg(rpc(
            DKGMessage::TimelockRequest(TimelockDKGRequest::SubtranscriptSignature(sig_req)),
            f.mempool.addrs[3],
            c.clone()
        ))
        .await
        .is_err());
    let missing = MissingTranscriptRequest::new(EPOCH, f.timelock.addrs[1]);
    assert!(mempool_mgr
        .process_peer_rpc_msg(rpc(
            DKGMessage::TimelockRequest(TimelockDKGRequest::MissingTranscript(missing)),
            f.mempool.addrs[3],
            c
        ))
        .await
        .is_err());
}

fn drain(pool: &VTxnPoolState) -> Vec<ValidatorTransaction> {
    pool.pull(
        std::time::Instant::now() + Duration::from_secs(1),
        10,
        1 << 30,
        TransactionFilter::empty(),
    )
}

#[tokio::test]
async fn two_managers_in_one_epoch_produce_separate_results() {
    let f = &*FIXTURE;
    // One vtxn pool per node, shared by both managers, as in the node.
    let pool = VTxnPoolState::default();
    let mut mempool_mgr = manager(&f.mempool, ChunkyDKGSessionKind::Mempool, pool.clone());
    let mut timelock_mgr = manager(&f.timelock, ChunkyDKGSessionKind::Timelock, pool.clone());

    // Both start from the same on-chain metadata, as the two start events of one reconfiguration.
    let event = || ChunkyDKGStartEvent {
        session_metadata: f.mempool.session_metadata.clone(),
        start_time_us: Duration::from_secs(1700000000).as_micros() as u64,
    };
    mempool_mgr.process_dkg_start_event(event()).await.unwrap();
    timelock_mgr.process_dkg_start_event(event()).await.unwrap();
    assert_eq!(mempool_mgr.state_name(), "AwaitSubtranscriptAggregation");
    assert_eq!(timelock_mgr.state_name(), "AwaitSubtranscriptAggregation");

    // The timelock manager dealt under the timelock id. The mempool manager under the mempool id.
    let c = Arc::new(RwLock::new(vec![]));
    for (mgr, setup, other) in [
        (&mut mempool_mgr, &f.mempool, &f.timelock),
        (&mut timelock_mgr, &f.timelock, &f.mempool),
    ] {
        let req = match setup.kind() {
            ChunkyDKGSessionKind::Mempool => {
                DKGMessage::ChunkyTranscriptRequest(ChunkyDKGTranscriptRequest::new(EPOCH))
            },
            ChunkyDKGSessionKind::Timelock => {
                TimelockTranscriptRequest(ChunkyDKGTranscriptRequest::new(EPOCH)).into()
            },
        };
        mgr.process_peer_rpc_msg(rpc(req, setup.addrs[3], c.clone()))
            .await
            .unwrap();
        let mut responses = std::mem::take(&mut *c.write());
        let Ok(DKGMessage::ChunkyTranscriptResponse(resp)) = responses.pop().unwrap() else {
            panic!("expected a transcript response");
        };
        let trx: ChunkyTranscript = bcs::from_bytes(&resp.transcript_bytes).unwrap();
        let mut rng = thread_rng();
        setup
            .dkg_config
            .verify_transcript(&trx, &setup.spks(), &mut rng)
            .expect("verifies under its own session");
        assert!(other
            .dkg_config
            .verify_transcript(&trx, &setup.spks(), &mut rng)
            .is_err());
    }

    // Aggregate and certify each in its own domain, then both put a result in the pool.
    for (mgr, setup, agg_with_hashes) in [
        (
            &mut mempool_mgr,
            &f.mempool,
            f.mempool.aggregate_subtranscripts_with_hashes(&[0, 1, 2]),
        ),
        (
            &mut timelock_mgr,
            &f.timelock,
            f.timelock.aggregate_subtranscripts_with_hashes(&[0, 1, 2]),
        ),
    ] {
        let agg = agg_with_hashes.aggregated_subtranscript.clone();
        mgr.process_aggregated_subtranscript(agg_with_hashes)
            .await
            .unwrap();
        let signature = sign(setup, setup.kind(), &agg, &[0, 1, 2]);
        mgr.process_certified_aggregated_subtranscript(CertifiedAggregatedSubtranscript {
            aggregated_subtranscript: Arc::new(agg),
            aggregate_signature: signature,
        })
        .await
        .unwrap();
        assert_eq!(mgr.state_name(), "Finished");
    }

    let pulled = drain(&pool);
    assert_eq!(
        pulled.len(),
        2,
        "one result per session, neither replaces the other"
    );
    let mempool_result = pulled
        .iter()
        .find(|t| matches!(t, ValidatorTransaction::ChunkyDKGResult(_)))
        .expect("mempool result");
    let timelock_result = pulled
        .iter()
        .find_map(|t| match t {
            ValidatorTransaction::TimelockDKGResult(o) => Some(o),
            _ => None,
        })
        .expect("timelock result");
    // The pool topics are separate and the timelock result passes the handler check.
    assert_ne!(Topic::ChunkyDKG, Topic::TimelockDKG);
    assert!(check_timelock_dkg_result(
        timelock_result,
        EPOCH,
        &f.timelock.session_metadata,
        ChainId::test(),
        &f.timelock.epoch_state.verifier,
        Some(local_hash()),
    )
    .is_ok());
    // The mempool result is untouched: its encryption key comes from the digest key, not the timelock secret.
    if let ValidatorTransaction::ChunkyDKGResult(out) = mempool_result {
        assert!(!out.encryption_key.is_empty());
        assert_ne!(
            out.certified_transcript.transcript_bytes,
            timelock_result.certified_transcript.transcript_bytes
        );
    }
    // The mpk of the timelock result is not derivable from the mempool aggregate.
    let cfg = chunky_threshold_config(&f.mempool.session_metadata);
    let mempool_mpk = derive_timelock_config(&f.mempool_agg.subtranscript, &cfg)
        .unwrap()
        .mpk;
    assert_ne!(timelock_result.mpk, mempool_mpk);
}

#[test]
fn unused_helpers_stay_used() {
    // Keeps the shared imports honest when a test above is filtered out.
    let _ = Uniform::generate_for_testing as fn() -> aptos_crypto::bls12381::PrivateKey;
}
