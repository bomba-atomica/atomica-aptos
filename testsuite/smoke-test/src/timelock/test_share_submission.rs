// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Phase 6: Share Submission Tests
//!
//! Tests for DK share submission by validators.
//!
//! Verifications:
//! - S1: Shares appear in TimelockState.shares[timelock_id]
//! - S2: Each share is 48 bytes (compressed G1)
//! - S3: Share count increases over time
//!
//! NOTE: These tests require validators to submit shares after
//! RequestRevealEvent, which needs the full validator-side implementation.

use super::test_helpers::{
    create_timelock_swarm, deadline_in_secs, force_block_production, register_timelock,
    wait_for_dk_reveal, wait_for_dkg_completion, TimelockTestConfig,
};
use aptos_forge::Swarm;
use aptos_logger::info;
use std::time::Duration;
use tokio::time::sleep;

/// Test: Verify shares appear after deadline passes.
///
/// Verification: S1
#[tokio::test]
async fn test_shares_appear_after_deadline() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing shares appear after deadline...");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    let deadline = deadline_in_secs(&client, 5).await;
    register_timelock(&swarm, &client, deadline)
        .await
        .expect("Registration should succeed");

    // Before deadline: no DK
    let dk_before = super::verify_dk_revealed(&client, 2).await;
    assert!(dk_before.is_err(), "DK should not exist before deadline");

    // Wait for deadline
    sleep(Duration::from_secs(6)).await;
    force_block_production(&swarm, &client).await.ok();

    // After deadline: shares submitted, DK revealed
    let dk_after = wait_for_dk_reveal(&client, 2, 60).await;

    match dk_after {
        Ok(dk) => {
            info!(
                "✅ Shares submitted and aggregated, DK available ({} bytes)",
                dk.len()
            );
        },
        Err(e) => {
            panic!(
                "❌ Shares not submitted or aggregated: {}. \
                 Validator share submission may not be implemented.",
                e
            );
        },
    }

    drop(swarm);
}

/// Test: Verify DK has correct size (48 bytes).
///
/// The aggregated DK should be a G1 point (48 bytes compressed).
///
/// Verification: S2 (indirectly through DK size)
#[tokio::test]
async fn test_dk_is_48_bytes() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing DK size is 48 bytes...");

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

    super::verify_dk_size(&dk).expect("DK should be 48 bytes");

    info!("✅ DK is {} bytes (G1 point)", dk.len());

    drop(swarm);
}

/// Test: Verify share count is sufficient for threshold.
///
/// With 3 validators, threshold is 3, so we need 3 shares.
///
/// Verification: S3
#[tokio::test]
async fn test_share_count_meets_threshold() {
    let config = TimelockTestConfig::with_validators(3);
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing share count meets threshold...");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    let deadline = deadline_in_secs(&client, 5).await;
    register_timelock(&swarm, &client, deadline)
        .await
        .expect("Registration should succeed");

    sleep(Duration::from_secs(6)).await;

    // Force multiple blocks to allow all validators to submit
    for _ in 0..10 {
        force_block_production(&swarm, &client).await.ok();
        sleep(Duration::from_millis(500)).await;
    }

    // If DK is revealed, threshold was met
    let dk = wait_for_dk_reveal(&client, 2, 60).await;

    match dk {
        Ok(_) => {
            let threshold = super::test_helpers::calculate_threshold(config.num_validators);
            info!(
                "✅ Threshold of {}/{} shares met (DK revealed)",
                threshold, config.num_validators
            );
        },
        Err(e) => {
            panic!("Threshold not met: {}", e);
        },
    }

    drop(swarm);
}
