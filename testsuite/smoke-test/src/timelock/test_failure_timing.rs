// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Failure Mode: Timing Failure Tests
//!
//! Tests for timing-related failure scenarios.

use super::test_helpers::{
    create_timelock_swarm, deadline_in_secs, register_timelock, wait_for_dkg_completion,
    TimelockTestConfig,
};
use aptos_forge::Swarm;
use aptos_logger::info;

/// Test: Verify no DK before deadline passes.
///
/// Expected: DK should not be available before deadline.
#[tokio::test]
async fn test_no_dk_before_deadline() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing no DK before deadline...");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    // Register with far future deadline (1 hour)
    let now = super::test_helpers::get_chain_time(&client).await;
    let deadline = now + 3600_000_000;

    register_timelock(&swarm, &client, deadline)
        .await
        .expect("Registration should succeed");

    // DK should NOT be available
    super::verify_dk_not_revealed(&client, 2)
        .await
        .expect("DK should NOT be revealed before deadline");

    info!("✅ No DK before deadline (correctly withheld)");

    drop(swarm);
}

/// Test: Verify DK query for non-existent timelock.
#[tokio::test]
async fn test_no_dk_nonexistent_timelock() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing no DK for non-existent timelock...");

    // Query for timelock ID 999 (never registered)
    let dk = super::verify_dk_revealed(&client, 999).await;

    assert!(dk.is_err(), "DK should not exist for non-existent timelock");

    info!("✅ Correctly returns None for non-existent timelock");

    drop(swarm);
}

/// Test: Verify deadline query for non-existent timelock.
#[tokio::test]
async fn test_no_deadline_nonexistent_timelock() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing no deadline for non-existent timelock...");

    // Query for timelock ID 999 (never registered)
    let deadline = super::verify_deadline_stored(&client, 999).await;

    assert!(
        deadline.is_err(),
        "Deadline should not exist for non-existent timelock"
    );

    info!("✅ Correctly returns None for non-existent timelock");

    drop(swarm);
}

/// Test: Verify DK query for registered but unexpired timelock.
#[tokio::test]
async fn test_no_dk_unexpired_timelock() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing no DK for unexpired timelock...");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    // Register with 5 minute deadline
    let deadline = deadline_in_secs(&client, 300).await;
    register_timelock(&swarm, &client, deadline)
        .await
        .expect("Registration should succeed");

    // Verify registration exists
    let stored = super::verify_deadline_stored(&client, 2)
        .await
        .expect("Deadline should be stored");
    assert_eq!(stored, deadline);

    // But DK should NOT exist
    super::verify_dk_not_revealed(&client, 2)
        .await
        .expect("DK should NOT be revealed for unexpired timelock");

    info!("✅ No DK for unexpired timelock (deadline in 5 min)");

    drop(swarm);
}
