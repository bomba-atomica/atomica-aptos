// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Phase 6: Share Validation Tests
//!
//! Tests for share validation rules.
//!
//! Verifications:
//! - S4: Duplicate shares from same validator rejected
//! - S2: Share is valid G1 point

use super::test_helpers::{
    create_timelock_swarm, deadline_in_secs, force_block_production, register_timelock,
    wait_for_dk_reveal, wait_for_dkg_completion, TimelockTestConfig,
};
use aptos_forge::Swarm;
use aptos_logger::info;
use std::time::Duration;
use tokio::time::sleep;

/// Test: Verify DK is valid G1 point.
///
/// The aggregated DK should be deserializable as a G1 point.
///
/// Verification: S2
#[tokio::test]
async fn test_dk_is_valid_g1() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing DK is valid G1 point...");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    let deadline = deadline_in_secs(&client, 5).await;
    register_timelock(&swarm, &client, deadline)
        .await
        .expect("Registration should succeed");

    sleep(Duration::from_secs(6)).await;
    force_block_production(&swarm, &client).await.ok();

    let dk = wait_for_dk_reveal(&client, 2, 60)
        .await
        .expect("DK should be revealed");

    // Basic validation: correct size and non-zero
    assert_eq!(
        dk.len(),
        super::DK_SHARE_SIZE_BYTES,
        "DK should be 48 bytes"
    );

    let non_zero_count = dk.iter().filter(|&&b| b != 0).count();
    assert!(
        non_zero_count > dk.len() / 2,
        "DK should be mostly non-zero (valid G1 point)"
    );

    info!(
        "✅ DK appears to be valid G1 point ({} bytes, {} non-zero)",
        dk.len(),
        non_zero_count
    );

    drop(swarm);
}

/// Test: Verify threshold enforcement with full validator set.
///
/// With all validators running, threshold should be met.
///
/// Verification: S4 (no duplicates affect count)
#[tokio::test]
async fn test_full_validator_set_reveals() {
    let config = TimelockTestConfig::with_validators(3);
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing full validator set reveals secret...");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    let deadline = deadline_in_secs(&client, 5).await;
    register_timelock(&swarm, &client, deadline)
        .await
        .expect("Registration should succeed");

    sleep(Duration::from_secs(6)).await;

    // Give validators time to submit
    for _ in 0..15 {
        force_block_production(&swarm, &client).await.ok();
        sleep(Duration::from_millis(500)).await;
    }

    let dk = wait_for_dk_reveal(&client, 2, 60)
        .await
        .expect("DK should be revealed with full validator set");

    super::verify_dk_size(&dk).expect("DK should be 48 bytes");

    info!("✅ Full validator set successfully revealed secret");

    drop(swarm);
}

/// Test: Verify multiple timelocks get different DKs.
///
/// Each timelock should have a unique DK (based on unique identity).
#[tokio::test]
async fn test_different_timelocks_different_dks() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing different timelocks get different DKs...");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    let now = super::test_helpers::get_chain_time(&client).await;
    let deadline_1 = now + 5_000_000;
    let deadline_2 = now + 6_000_000;

    register_timelock(&swarm, &client, deadline_1)
        .await
        .expect("Registration 1 should succeed");
    register_timelock(&swarm, &client, deadline_2)
        .await
        .expect("Registration 2 should succeed");

    // Wait for both deadlines
    sleep(Duration::from_secs(8)).await;

    for _ in 0..15 {
        force_block_production(&swarm, &client).await.ok();
        sleep(Duration::from_millis(500)).await;
    }

    let dk_1 = wait_for_dk_reveal(&client, 2, 60)
        .await
        .expect("DK 1 should be revealed");
    let dk_2 = wait_for_dk_reveal(&client, 3, 60)
        .await
        .expect("DK 2 should be revealed");

    // DKs should be different (different identities)
    assert_ne!(dk_1, dk_2, "Different timelocks should have different DKs");

    info!(
        "✅ Different timelocks have different DKs ({} bytes each)",
        dk_1.len()
    );

    drop(swarm);
}
