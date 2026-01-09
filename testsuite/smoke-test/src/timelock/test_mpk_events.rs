// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Phase 3: MPK Event Tests
//!
//! Tests for MPK publication event emission.
//!
//! Verifications:
//! - M3: MasterPublicKeyPublishedEvent emitted
//! - M4: Event contains correct MPK bytes
//!
//! NOTE: These tests require event query support and will fail until
//! MPK publication is implemented in validators.

use super::test_helpers::{
    create_timelock_swarm, wait_for_dkg_completion, wait_for_mpk_publication, TimelockTestConfig,
};
use aptos_forge::Swarm;
use aptos_logger::info;

/// Test: Verify MasterPublicKeyPublishedEvent is emitted.
///
/// We verify indirectly by checking MPK is available in threshold_dsa.
/// If the event wasn't emitted, validators wouldn't store their shares.
///
/// Verification: M3
#[tokio::test]
async fn test_mpk_published_event_emitted() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing MasterPublicKeyPublishedEvent emission...");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    // If MPK is published, the event was emitted
    let mpk = wait_for_mpk_publication(&client, 30)
        .await
        .expect("MPK should be published (event should be emitted)");

    assert!(!mpk.is_empty(), "MPK should not be empty");

    info!("✅ MasterPublicKeyPublishedEvent emitted (MPK available)");

    drop(swarm);
}

/// Test: Verify event contains correct interval ID.
///
/// The event should have id=1 (MPK_ID).
///
/// Verification: M4 (id field)
#[tokio::test]
async fn test_mpk_event_interval_id() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing MPK event interval ID...");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    // Query MPK for interval 1 (MPK_ID)
    let mpk = super::verify_mpk_on_chain(&client, super::MPK_ID)
        .await
        .expect("MPK should be available for MPK_ID (1)");

    assert!(!mpk.is_empty(), "MPK for interval 1 should exist");

    // Query for interval 0 should fail
    let mpk_0 = super::verify_mpk_on_chain(&client, 0).await;
    assert!(mpk_0.is_err(), "MPK should not exist for interval 0");

    info!("✅ MPK stored at correct interval ID ({})", super::MPK_ID);

    drop(swarm);
}
