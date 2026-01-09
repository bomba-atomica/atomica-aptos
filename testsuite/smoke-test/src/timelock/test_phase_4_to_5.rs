// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Integration Test: Phase 4 → Phase 5 (Registration → Deadline Detection)
//!
//! Tests that verify registration leads to deadline detection.

use super::test_helpers::{
    create_timelock_swarm, deadline_in_secs, force_block_production, register_timelock,
    wait_for_dk_reveal, wait_for_dkg_completion, TimelockTestConfig,
};
use aptos_forge::Swarm;
use aptos_logger::info;
use std::time::Duration;
use tokio::time::sleep;

/// Test: Verify registration to reveal event flow.
///
/// Registration with a short deadline should eventually trigger RequestRevealEvent.
///
/// Phases: 4 → 5
#[tokio::test]
async fn test_registration_to_reveal_event() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("[Phase 4→5] Testing registration to reveal event...");

    // Wait for DKG first
    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    // Phase 4: Register with short deadline
    let deadline = deadline_in_secs(&client, 5).await;
    register_timelock(&swarm, &client, deadline)
        .await
        .expect("Registration should succeed");

    // Verify registration
    let stored = super::verify_deadline_stored(&client, 2)
        .await
        .expect("Deadline should be stored");
    assert_eq!(stored, deadline);

    info!("Registered timelock ID 2, waiting for deadline...");

    // Phase 5: Wait for deadline and trigger reveal
    sleep(Duration::from_secs(6)).await;

    for _ in 0..10 {
        force_block_production(&swarm, &client).await.ok();
        sleep(Duration::from_millis(500)).await;
    }

    // If RequestRevealEvent was emitted, we should eventually see DK
    let dk_result = wait_for_dk_reveal(&client, 2, 60).await;

    match dk_result {
        Ok(dk) => {
            info!(
                "✅ [Phase 4→5] Registration → Reveal event → DK ({} bytes)",
                dk.len()
            );
        },
        Err(e) => {
            panic!("❌ [Phase 4→5] Reveal flow not working: {}", e);
        },
    }

    drop(swarm);
}

/// Test: Verify multiple registrations are tracked.
///
/// Phases: 4 → 5
#[tokio::test]
async fn test_multiple_registrations_tracked() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("[Phase 4→5] Testing multiple registrations tracked...");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    let now = super::test_helpers::get_chain_time(&client).await;

    // Register 3 timelocks with different deadlines
    for i in 0..3 {
        let deadline = now + (60 + i * 10) * 1_000_000; // 60s, 70s, 80s
        register_timelock(&swarm, &client, deadline)
            .await
            .expect("Registration should succeed");
    }

    // Verify all are stored
    for id in 2..=4 {
        let stored = super::verify_deadline_stored(&client, id)
            .await
            .expect(&format!("ID {} should exist", id));
        info!("ID {} has deadline {}", id, stored);
    }

    info!("✅ [Phase 4→5] Multiple registrations tracked correctly");

    drop(swarm);
}
