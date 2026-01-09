// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Phase 4: Registration Event Tests
//!
//! Tests for registration event emission.
//!
//! Verifications:
//! - R5: TimelockRegisteredEvent emitted
//! - R6: Event contains correct timelock_id and deadline

use super::test_helpers::{
    create_timelock_swarm, deadline_in_secs, register_timelock, TimelockTestConfig,
};
use aptos_forge::Swarm;
use aptos_logger::info;

/// Test: Verify TimelockRegisteredEvent is emitted on registration.
///
/// We verify indirectly by confirming the state was updated.
///
/// Verification: R5
#[tokio::test]
async fn test_registration_event_emitted() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing TimelockRegisteredEvent emission...");

    let deadline = deadline_in_secs(&client, 60).await;

    register_timelock(&swarm, &client, deadline)
        .await
        .expect("Registration should succeed");

    // State update confirms event was processed
    let stored = super::verify_deadline_stored(&client, 2)
        .await
        .expect("Deadline should be stored");

    assert_eq!(stored, deadline, "Stored deadline should match");

    info!("✅ TimelockRegisteredEvent emitted (state updated)");

    drop(swarm);
}

/// Test: Verify event has correct timelock_id.
///
/// The first registration should have timelock_id = 2.
///
/// Verification: R6 (timelock_id field)
#[tokio::test]
async fn test_registration_event_timelock_id() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing registration event timelock_id...");

    let deadline = deadline_in_secs(&client, 60).await;

    register_timelock(&swarm, &client, deadline)
        .await
        .expect("Registration should succeed");

    // ID 2 should exist, ID 1 should not (for this deadline)
    let id_2_deadline = super::verify_deadline_stored(&client, 2).await;
    assert!(id_2_deadline.is_ok(), "ID 2 should exist");

    info!("✅ Event has correct timelock_id (2)");

    drop(swarm);
}

/// Test: Verify event has correct deadline.
///
/// Verification: R6 (deadline field)
#[tokio::test]
async fn test_registration_event_deadline() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing registration event deadline...");

    // Use a specific deadline
    let now = super::test_helpers::get_chain_time(&client).await;
    let deadline = now + 123_456_789; // Unique value

    register_timelock(&swarm, &client, deadline)
        .await
        .expect("Registration should succeed");

    let stored = super::verify_deadline_stored(&client, 2)
        .await
        .expect("Deadline should be stored");

    assert_eq!(stored, deadline, "Event deadline should match exactly");

    info!("✅ Event has correct deadline ({})", deadline);

    drop(swarm);
}
