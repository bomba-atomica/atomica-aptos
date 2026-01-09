// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Integration Test: Phase 5 → Phase 7 (Deadline → Aggregation)
//!
//! Tests that verify deadline triggers share submission and aggregation.

use super::test_helpers::{
    create_timelock_swarm, deadline_in_secs, force_block_production, register_timelock,
    wait_for_dk_reveal, wait_for_dkg_completion, TimelockTestConfig,
};
use aptos_forge::Swarm;
use aptos_logger::info;
use std::time::Duration;
use tokio::time::sleep;

/// Test: Verify reveal event to DK available flow.
///
/// RequestRevealEvent should trigger validators to submit shares,
/// which should aggregate to produce the DK.
///
/// Phases: 5 → 6 → 7
#[tokio::test]
async fn test_reveal_event_to_dk_available() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("[Phase 5→7] Testing reveal event to DK available...");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    let deadline = deadline_in_secs(&client, 5).await;
    register_timelock(&swarm, &client, deadline)
        .await
        .expect("Registration should succeed");

    // Before deadline: no DK
    super::verify_dk_not_revealed(&client, 2)
        .await
        .expect("DK should not exist before deadline");

    info!("Waiting for deadline to trigger reveal flow...");

    // Phase 5: Deadline passes
    sleep(Duration::from_secs(6)).await;

    // Phase 6: Validators submit shares (triggered by RequestRevealEvent)
    // Phase 7: Shares aggregate to DK
    for _ in 0..20 {
        force_block_production(&swarm, &client).await.ok();
        sleep(Duration::from_millis(500)).await;
    }

    let dk = wait_for_dk_reveal(&client, 2, 60)
        .await
        .expect("DK should be available after aggregation");

    super::verify_dk_size(&dk).expect("DK should be 48 bytes");

    info!("✅ [Phase 5→7] Deadline → Shares → DK ({} bytes)", dk.len());

    drop(swarm);
}

/// Test: Verify complete reveal flow timing.
///
/// Phases: 5 → 6 → 7
#[tokio::test]
async fn test_reveal_flow_timing() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("[Phase 5→7] Testing reveal flow timing...");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    let deadline = deadline_in_secs(&client, 5).await;
    register_timelock(&swarm, &client, deadline)
        .await
        .expect("Registration should succeed");

    sleep(Duration::from_secs(6)).await;

    let start = std::time::Instant::now();

    for i in 0..60 {
        force_block_production(&swarm, &client).await.ok();

        if super::verify_dk_revealed(&client, 2).await.is_ok() {
            let elapsed = start.elapsed();
            info!(
                "✅ [Phase 5→7] DK revealed in {} iterations ({:.2}s)",
                i + 1,
                elapsed.as_secs_f64()
            );
            return;
        }

        sleep(Duration::from_millis(500)).await;
    }

    panic!("DK not revealed within 30s after deadline");
}
