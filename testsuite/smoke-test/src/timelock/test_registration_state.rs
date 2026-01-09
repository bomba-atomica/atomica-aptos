// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Phase 4: Registration State Tests
//!
//! Tests for registration state updates.
//!
//! Verifications:
//! - R3: id_to_deadline mapping updated
//! - R4: Deadline in pending_deadlines

use super::test_helpers::{
    create_timelock_swarm, deadline_in_secs, register_timelock, TimelockTestConfig,
};
use aptos_forge::Swarm;
use aptos_logger::info;

/// Test: Verify deadline is stored correctly.
///
/// Verification: R3
#[tokio::test]
async fn test_deadline_stored_correctly() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing deadline stored correctly...");

    let deadline = deadline_in_secs(&client, 120).await;

    register_timelock(&swarm, &client, deadline)
        .await
        .expect("Registration should succeed");

    let stored_deadline = super::verify_deadline_stored(&client, 2)
        .await
        .expect("Deadline should be stored");

    assert_eq!(
        stored_deadline, deadline,
        "Stored deadline should exactly match registered deadline"
    );

    info!(
        "✅ Deadline stored correctly: {} == {}",
        stored_deadline, deadline
    );

    drop(swarm);
}

/// Test: Verify multiple timelocks can share the same deadline.
///
/// Two registrations with identical deadlines should both be tracked.
///
/// Verification: R3, R4
#[tokio::test]
async fn test_multiple_timelocks_same_deadline() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing multiple timelocks with same deadline...");

    let deadline = deadline_in_secs(&client, 60).await;

    // Register two timelocks with the same deadline
    register_timelock(&swarm, &client, deadline)
        .await
        .expect("First registration should succeed");

    register_timelock(&swarm, &client, deadline)
        .await
        .expect("Second registration should succeed");

    // Both should have the same deadline stored
    let deadline_2 = super::verify_deadline_stored(&client, 2)
        .await
        .expect("ID 2 should exist");
    let deadline_3 = super::verify_deadline_stored(&client, 3)
        .await
        .expect("ID 3 should exist");

    assert_eq!(deadline_2, deadline, "ID 2 should have correct deadline");
    assert_eq!(deadline_3, deadline, "ID 3 should have correct deadline");
    assert_eq!(deadline_2, deadline_3, "Both should have the same deadline");

    info!("✅ Multiple timelocks share deadline correctly");

    drop(swarm);
}

/// Test: Verify different deadlines are stored independently.
///
/// Verification: R3
#[tokio::test]
async fn test_different_deadlines_stored() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing different deadlines stored independently...");

    let deadline_1 = deadline_in_secs(&client, 60).await;
    let deadline_2 = deadline_in_secs(&client, 120).await;
    let deadline_3 = deadline_in_secs(&client, 180).await;

    register_timelock(&swarm, &client, deadline_1)
        .await
        .expect("Registration 1 should succeed");
    register_timelock(&swarm, &client, deadline_2)
        .await
        .expect("Registration 2 should succeed");
    register_timelock(&swarm, &client, deadline_3)
        .await
        .expect("Registration 3 should succeed");

    // Verify each ID has its correct deadline
    let stored_1 = super::verify_deadline_stored(&client, 2)
        .await
        .expect("ID 2 should exist");
    let stored_2 = super::verify_deadline_stored(&client, 3)
        .await
        .expect("ID 3 should exist");
    let stored_3 = super::verify_deadline_stored(&client, 4)
        .await
        .expect("ID 4 should exist");

    assert_eq!(stored_1, deadline_1, "ID 2 deadline mismatch");
    assert_eq!(stored_2, deadline_2, "ID 3 deadline mismatch");
    assert_eq!(stored_3, deadline_3, "ID 4 deadline mismatch");

    // Verify they're all different
    assert!(stored_1 < stored_2, "Deadlines should be in order");
    assert!(stored_2 < stored_3, "Deadlines should be in order");

    info!("✅ Different deadlines stored correctly");

    drop(swarm);
}
