// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Stop-rule prototype for the timelock chunky DKG session (node-dkg-chunky-wiring).
//!
//! Two chunky DKG sessions run for the same dealers and target validator set in one epoch.
//! One is bound to the mempool session id (`ChunkyDKGSessionMetadata`), the other to
//! `TimelockDKGSessionId`. The tests show that the aptos-dkg library needs no change for this:
//! the sessions complete independently, their master public keys are unrelated, and a dealing
//! made for one session fails verification under the other.

use crate::chunky::test_utils::ChunkyTestSetup;
use aptos_crypto::Uniform;
use aptos_dkg::pvss::{
    traits::{
        transcript::{Aggregatable, HasAggregatableSubtranscript},
        Transcript, TranscriptCore,
    },
    Player,
};
use aptos_types::{
    chain_id::ChainId,
    dkg::chunky_dkg::{ChunkyInputSecret, ChunkyTranscript, TimelockDKGSessionId},
};
use rand::{prelude::StdRng, thread_rng, SeedableRng};
use serde::Serialize;

fn deal_under<A: Serialize + Clone>(
    setup: &ChunkyTestSetup,
    sid: &A,
    dealer_index: usize,
) -> ChunkyTranscript {
    let mut rng = StdRng::from_rng(thread_rng()).unwrap();
    let input_secret = ChunkyInputSecret::generate(&mut rng);
    ChunkyTranscript::deal(
        &setup.dkg_config.threshold_config,
        &setup.dkg_config.public_parameters,
        &setup.private_keys[dealer_index],
        &setup.public_keys[dealer_index],
        &setup.dkg_config.eks,
        &input_secret,
        sid,
        &Player { id: dealer_index },
        &mut rng,
    )
}

fn verify_under<A: Serialize + Clone>(
    setup: &ChunkyTestSetup,
    trx: &ChunkyTranscript,
    sid: &A,
) -> anyhow::Result<()> {
    let mut rng = thread_rng();
    trx.verify(
        &setup.dkg_config.threshold_config,
        &setup.dkg_config.public_parameters,
        &setup.spks(),
        &setup.dkg_config.eks,
        sid,
        &mut rng,
    )
}

#[test]
fn two_sessions_one_reconfiguration_distinct_ids_unrelated_mpk() {
    let setup = ChunkyTestSetup::new_uniform(4);
    let mempool_sid = setup.session_metadata.clone();
    let timelock_sid = TimelockDKGSessionId::new(ChainId::test(), setup.session_metadata.clone());
    assert_ne!(
        bcs::to_bytes(&mempool_sid).unwrap(),
        bcs::to_bytes(&timelock_sid).unwrap()
    );

    let dealers = [0usize, 1, 2];
    let mempool_trxs: Vec<_> = dealers
        .iter()
        .map(|&i| deal_under(&setup, &mempool_sid, i))
        .collect();
    let timelock_trxs: Vec<_> = dealers
        .iter()
        .map(|&i| deal_under(&setup, &timelock_sid, i))
        .collect();

    // Each session verifies under its own id.
    for t in &mempool_trxs {
        verify_under(&setup, t, &mempool_sid).expect("mempool dealing under mempool id");
    }
    for t in &timelock_trxs {
        verify_under(&setup, t, &timelock_sid).expect("timelock dealing under timelock id");
    }

    // A dealing for one session fails under the other, in both directions.
    for t in &mempool_trxs {
        assert!(verify_under(&setup, t, &timelock_sid).is_err());
    }
    for t in &timelock_trxs {
        assert!(verify_under(&setup, t, &mempool_sid).is_err());
    }

    // Both sessions aggregate to a subtranscript with their own, unrelated mpk.
    let sc = &setup.dkg_config.threshold_config;
    let mempool_agg = Aggregatable::aggregate(
        sc,
        mempool_trxs.iter().map(|t| t.get_subtranscript()).collect(),
    )
    .unwrap();
    let timelock_agg = Aggregatable::aggregate(
        sc,
        timelock_trxs
            .iter()
            .map(|t| t.get_subtranscript())
            .collect(),
    )
    .unwrap();
    let mempool_mpk = mempool_agg.get_dealt_public_key().as_g2();
    let timelock_mpk = timelock_agg.get_dealt_public_key().as_g2();
    assert_ne!(mempool_mpk, timelock_mpk);
}
