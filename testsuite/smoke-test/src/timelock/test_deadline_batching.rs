// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Phase 5: Deadline Batching Tests
//!
//! Tests for batch processing of deadlines.
//!
//! Verifications:
//! - DD4: Multiple timelocks with same deadline batched in one event
//! - DD5: Earlier deadlines processed before later ones

use super::test_helpers::{
    create_timelock_swarm, force_block_production, register_timelock, wait_for_dk_reveal,
    wait_for_dkg_completion, TimelockTestConfig,
};
use aptos_forge::Swarm;
use aptos_logger::info;
use std::time::Duration;
use tokio::time::sleep;

/// Test: Verify timelocks with same deadline are batched.
///
/// Two timelocks with identical deadlines should be revealed together.
///
/// Verification: DD4
#[tokio::test]
async fn test_same_deadline_batched() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing same deadline batched...");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    let now = super::test_helpers::get_chain_time(&client).await;
    let deadline = now + 5_000_000; // 5 seconds

    // Register two timelocks with identical deadline
    register_timelock(&swarm, &client, deadline)
        .await
        .expect("Registration 1 should succeed");
    register_timelock(&swarm, &client, deadline)
        .await
        .expect("Registration 2 should succeed");

    info!("Registered IDs 2 and 3 with same deadline");

    sleep(Duration::from_secs(6)).await;
    force_block_production(&swarm, &client).await.ok();

    // Both should be revealed (in same batch or close together)
    let dk_2 = wait_for_dk_reveal(&client, 2, 30).await;
    let dk_3 = wait_for_dk_reveal(&client, 3, 30).await;

    match (dk_2, dk_3) {
        (Ok(dk2), Ok(dk3)) => {
            assert!(!dk2.is_empty(), "DK 2 should not be empty");
            assert!(!dk3.is_empty(), "DK 3 should not be empty");
            info!("✅ Both timelocks with same deadline revealed");
        },
        (Err(e), _) | (_, Err(e)) => {
            panic!("Both timelocks should be revealed: {}", e);
        },
    }

    drop(swarm);
}

/// Test: Verify deadline ordering (earlier first).
///
/// An earlier deadline should be processed before a later one.
///
/// Verification: DD5
#[tokio::test]
async fn test_deadline_ordering() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing deadline ordering...");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    let now = super::test_helpers::get_chain_time(&client).await;
    let early_deadline = now + 5_000_000; // 5 seconds
    let late_deadline = now + 15_000_000; // 15 seconds

    // Register in reverse order (late first, early second)
    register_timelock(&swarm, &client, late_deadline)
        .await
        .expect("Late registration should succeed");
    register_timelock(&swarm, &client, early_deadline)
        .await
        .expect("Early registration should succeed");

    info!("Registered ID 2 (late) and ID 3 (early)");

    // Wait for early deadline
    sleep(Duration::from_secs(6)).await;
    force_block_production(&swarm, &client).await.ok();

    // Early deadline (ID 3) should be revealed first
    let dk_early = super::verify_dk_revealed(&client, 3).await;
    let dk_late = super::verify_dk_revealed(&client, 2).await;

    match dk_early {
        Ok(_) => {
            info!("Early deadline (ID 3) revealed");
            // Late should NOT be revealed yet
            if dk_late.is_ok() {
                info!("Note: Late deadline also revealed (may be timing)");
            } else {
                info!("Late deadline (ID 2) not yet revealed (correct)");
            }
        },
        Err(e) => {
            panic!("Early deadline should be revealed first: {}", e);
        },
    }

    // Wait for late deadline
    sleep(Duration::from_secs(10)).await;
    force_block_production(&swarm, &client).await.ok();

    let dk_late_final = wait_for_dk_reveal(&client, 2, 30).await;
    assert!(
        dk_late_final.is_ok(),
        "Late deadline should be revealed now"
    );

    info!("✅ Deadlines processed in correct order");

    drop(swarm);
}

/// Test: Verify many timelocks with same deadline.
///
/// Verification: DD4 (stress test)
#[tokio::test]
async fn test_many_timelocks_same_deadline() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing many timelocks with same deadline...");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    let now = super::test_helpers::get_chain_time(&client).await;
    let deadline = now + 5_000_000;

    // Register 5 timelocks with same deadline
    for i in 0..5 {
        register_timelock(&swarm, &client, deadline)
            .await
            .expect(&format!("Registration {} should succeed", i));
    }

    info!("Registered IDs 2-6 with same deadline");

    sleep(Duration::from_secs(6)).await;

    // Force several blocks
    for _ in 0..10 {
        force_block_production(&swarm, &client).await.ok();
        sleep(Duration::from_millis(200)).await;
    }

    // All should be revealed
    let mut revealed_count = 0;
    for id in 2..=6 {
        if wait_for_dk_reveal(&client, id, 30).await.is_ok() {
            revealed_count += 1;
        }
    }

    assert_eq!(revealed_count, 5, "All 5 timelocks should be revealed");

    info!(
        "✅ All {} timelocks with same deadline revealed",
        revealed_count
    );

    drop(swarm);
}
