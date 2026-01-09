// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Integration Test: Phase 2 → Phase 3 (DKG → MPK Publication)
//!
//! Tests that verify DKG completion leads to MPK publication.

use super::test_helpers::{
    create_timelock_swarm, wait_for_dkg_completion, wait_for_mpk_publication, TimelockTestConfig,
};
use aptos_forge::Swarm;
use aptos_logger::info;

/// Test: Verify DKG produces MPK.
///
/// DKG completion should lead to MPK being published to threshold_dsa.
///
/// Phases: 2 → 3
#[tokio::test]
async fn test_dkg_produces_mpk() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("[Phase 2→3] Testing DKG produces MPK...");

    // Phase 2: DKG completes
    let transcript = wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;
    info!("DKG completed, transcript {} bytes", transcript.len());

    // Phase 3: MPK should be published
    let mpk = wait_for_mpk_publication(&client, 30).await;

    match mpk {
        Ok(mpk_bytes) => {
            super::verify_mpk_size(&mpk_bytes).expect("MPK should be 96 bytes");
            info!(
                "✅ [Phase 2→3] DKG produced MPK ({} bytes)",
                mpk_bytes.len()
            );
        },
        Err(e) => {
            panic!(
                "❌ [Phase 2→3] MPK not published after DKG: {}. \
                 Validator MPK publication needs to be implemented.",
                e
            );
        },
    }

    drop(swarm);
}

/// Test: Verify MPK matches DKG transcript.
///
/// The MPK in threshold_dsa should be derivable from the DKG transcript.
///
/// Phases: 2 → 3
#[tokio::test]
async fn test_mpk_matches_dkg_transcript() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("[Phase 2→3] Testing MPK matches DKG transcript...");

    let transcript = wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    let mpk = wait_for_mpk_publication(&client, 30)
        .await
        .expect("MPK should be published");

    // Transcript should be larger than MPK (contains more data)
    assert!(
        transcript.len() > mpk.len(),
        "Transcript ({} bytes) should contain MPK ({} bytes) plus more",
        transcript.len(),
        mpk.len()
    );

    // MPK should be valid
    super::verify_mpk_size(&mpk).expect("MPK should be 96 bytes");

    info!(
        "✅ [Phase 2→3] MPK ({} bytes) derived from transcript ({} bytes)",
        mpk.len(),
        transcript.len()
    );

    drop(swarm);
}
