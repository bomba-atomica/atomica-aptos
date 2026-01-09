// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Phase 4: Registration ID Tests
//!
//! Tests for timelock ID assignment.
//!
//! Verifications:
//! - R1: First registration returns ID 2
//! - R2: Sequential registrations increment ID

use super::test_helpers::{
    create_timelock_swarm, deadline_in_secs, register_timelock, TimelockTestConfig,
};
use aptos_forge::Swarm;
use aptos_logger::info;

/// Test: Verify first registration returns ID 2.
///
/// ID 1 is reserved for MPK_ID.
///
/// Verification: R1
#[tokio::test]
async fn test_first_registration_returns_id_2() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing first registration returns ID 2...");

    let deadline = deadline_in_secs(&client, 60).await;

    register_timelock(&swarm, &client, deadline)
        .await
        .expect("Registration should succeed");

    // Verify ID 2 has the registered deadline
    let stored_deadline = super::verify_deadline_stored(&client, 2)
        .await
        .expect("Deadline should be stored for ID 2");

    assert_eq!(stored_deadline, deadline, "Stored deadline should match");

    // Verify ID 1 doesn't have this deadline
    let id_1 = super::verify_deadline_stored(&client, 1).await;
    assert!(
        id_1.is_err() || id_1.unwrap() != deadline,
        "ID 1 should not have user registration"
    );

    info!("✅ First registration assigned ID 2");

    drop(swarm);
}

/// Test: Verify sequential ID assignment.
///
/// Registrations should get IDs 2, 3, 4, ...
///
/// Verification: R2
#[tokio::test]
async fn test_sequential_id_assignment() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing sequential ID assignment...");

    let base_deadline = deadline_in_secs(&client, 60).await;

    // Register 5 timelocks with different deadlines
    let deadlines: Vec<u64> = (0..5).map(|i| base_deadline + i * 1_000_000).collect();

    for (i, &deadline) in deadlines.iter().enumerate() {
        register_timelock(&swarm, &client, deadline)
            .await
            .expect(&format!("Registration {} should succeed", i + 1));

        let expected_id = 2 + i as u64;
        let stored_deadline = super::verify_deadline_stored(&client, expected_id)
            .await
            .expect(&format!("Deadline should be stored for ID {}", expected_id));

        assert_eq!(
            stored_deadline, deadline,
            "ID {} should have correct deadline",
            expected_id
        );

        info!("  ID {} assigned correctly", expected_id);
    }

    // Verify IDs 2-6 all exist
    for id in 2..=6 {
        let result = super::verify_deadline_stored(&client, id).await;
        assert!(result.is_ok(), "ID {} should exist", id);
    }

    // Verify ID 7 doesn't exist yet
    let id_7 = super::verify_deadline_stored(&client, 7).await;
    assert!(id_7.is_err(), "ID 7 should not exist");

    info!("✅ Sequential IDs 2-6 assigned correctly");

    drop(swarm);
}

/// Test: Verify concurrent registrations get unique IDs.
///
/// Even with rapid registrations, each should get a unique ID.
///
/// Verification: R2 (concurrent case)
#[tokio::test]
async fn test_concurrent_registration_unique_ids() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing concurrent registration unique IDs...");

    let base_deadline = deadline_in_secs(&client, 60).await;

    // Register 3 timelocks in quick succession
    for i in 0..3 {
        let deadline = base_deadline + i * 1_000_000;
        register_timelock(&swarm, &client, deadline)
            .await
            .expect("Registration should succeed");
    }

    // All 3 should have unique IDs (2, 3, 4)
    let mut found_deadlines = vec![];
    for id in 2..=4 {
        let deadline = super::verify_deadline_stored(&client, id)
            .await
            .expect(&format!("ID {} should exist", id));
        found_deadlines.push(deadline);
    }

    // Verify all deadlines are different
    found_deadlines.sort();
    found_deadlines.dedup();
    assert_eq!(
        found_deadlines.len(),
        3,
        "All 3 registrations should have unique IDs with unique deadlines"
    );

    info!("✅ Concurrent registrations got unique IDs");

    drop(swarm);
}
