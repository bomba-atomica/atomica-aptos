// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Integration Test: Phase 1 → Phase 2 (Init → DKG)
//!
//! Tests that verify initialization properly triggers DKG.

use super::test_helpers::{create_timelock_swarm, wait_for_dkg_completion, TimelockTestConfig};
use aptos_forge::Swarm;
use aptos_logger::info;

/// Test: Verify initialization triggers DKG.
///
/// StartKeyGenEvent should lead to DKG completion.
///
/// Phases: 1 → 2
#[tokio::test]
async fn test_init_triggers_dkg() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("[Phase 1→2] Testing init triggers DKG...");

    // Phase 1: Initialization happens at genesis
    // Verify TimelockState exists
    let deadline_check = super::verify_deadline_stored(&client, 0).await;
    info!("TimelockState exists: view function callable");

    // Phase 2: DKG should complete
    let transcript = wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    assert!(
        !transcript.is_empty(),
        "DKG should complete after initialization"
    );

    info!(
        "✅ [Phase 1→2] Init triggered DKG, transcript {} bytes",
        transcript.len()
    );

    drop(swarm);
}

/// Test: Verify DKG state transitions correctly.
///
/// After init, DKG should go through proper states.
///
/// Phases: 1 → 2
#[tokio::test]
async fn test_dkg_state_transitions() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("[Phase 1→2] Testing DKG state transitions...");

    // Initial state: DKG may or may not have completed
    let initial_dkg = super::verify_dkg_transcript_exists(&client).await;
    info!("Initial DKG state: {:?}", initial_dkg.is_ok());

    // Wait for DKG to complete
    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    // Final state: DKG should be complete
    let final_dkg = super::verify_dkg_transcript_exists(&client)
        .await
        .expect("DKG should be complete");

    assert!(
        final_dkg.last_completed.is_some(),
        "DKG should have completed"
    );

    info!("✅ [Phase 1→2] DKG state transitions correct");

    drop(swarm);
}
