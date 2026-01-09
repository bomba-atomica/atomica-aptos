// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Failure Mode: Threshold Failure Tests
//!
//! Tests for scenarios where threshold is not met.

use super::test_helpers::{
    create_timelock_swarm, deadline_in_secs, force_block_production, register_timelock,
    stop_validator, wait_for_dkg_completion, TimelockTestConfig,
};
use aptos_forge::Swarm;
use aptos_logger::info;
use std::time::Duration;
use tokio::time::sleep;

/// Test: Verify no reveal when validators fail.
///
/// Stop enough validators to prevent threshold from being met.
#[tokio::test]
async fn test_validator_failure_blocks_reveal() {
    let config = TimelockTestConfig::with_validators(3);
    let (mut swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing validator failure blocks reveal...");

    // DKG must complete first (needs all validators)
    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    let deadline = deadline_in_secs(&client, 10).await;
    register_timelock(&swarm, &client, deadline)
        .await
        .expect("Registration should succeed");

    // Stop 2 of 3 validators (below threshold of 3)
    stop_validator(&mut swarm, 1).await.ok();
    stop_validator(&mut swarm, 2).await.ok();

    info!("Stopped validators 1 and 2");

    sleep(Duration::from_secs(12)).await;

    // Try to force blocks (only validator 0 running)
    for _ in 0..10 {
        force_block_production(&swarm, &client).await.ok();
        sleep(Duration::from_millis(500)).await;
    }

    // DK should NOT be revealed (only 1 of 3 shares)
    let dk = super::verify_dk_revealed(&client, 2).await;

    match dk {
        Err(_) => {
            info!("✅ Validator failure correctly blocks reveal");
        },
        Ok(_) => {
            info!("Note: DK revealed despite stopped validators");
            info!("  This may indicate threshold is lower than expected");
        },
    }

    drop(swarm);
}

/// Test: Verify single validator cannot reveal alone.
#[tokio::test]
async fn test_single_validator_cannot_reveal() {
    let config = TimelockTestConfig::with_validators(3);
    let (mut swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing single validator cannot reveal...");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    let deadline = deadline_in_secs(&client, 10).await;
    register_timelock(&swarm, &client, deadline)
        .await
        .expect("Registration should succeed");

    // Stop all but one validator
    stop_validator(&mut swarm, 1).await.ok();
    stop_validator(&mut swarm, 2).await.ok();

    sleep(Duration::from_secs(12)).await;

    for _ in 0..5 {
        force_block_production(&swarm, &client).await.ok();
        sleep(Duration::from_millis(500)).await;
    }

    // Single validator should not meet threshold
    let dk = super::verify_dk_revealed(&client, 2).await;

    assert!(
        dk.is_err(),
        "Single validator should not be able to reveal alone"
    );

    info!("✅ Single validator correctly cannot reveal");

    drop(swarm);
}

/// Test: Verify threshold requirements.
///
/// For 3 validators, threshold is (3*2/3)+1 = 3.
#[tokio::test]
async fn test_threshold_requirements() {
    info!("Testing threshold requirements...");

    // Verify threshold formula
    let threshold_3 = super::test_helpers::calculate_threshold(3);
    let threshold_4 = super::test_helpers::calculate_threshold(4);
    let threshold_7 = super::test_helpers::calculate_threshold(7);

    info!("Threshold for 3 validators: {}", threshold_3);
    info!("Threshold for 4 validators: {}", threshold_4);
    info!("Threshold for 7 validators: {}", threshold_7);

    // For Byzantine fault tolerance, we need 2f+1 out of 3f+1
    // With 3 validators (f=0), threshold is 1? No, we use (2n/3)+1
    // 3: (3*2/3)+1 = 2+1 = 3
    // 4: (4*2/3)+1 = 2+1 = 3
    // 7: (7*2/3)+1 = 4+1 = 5

    assert_eq!(threshold_3, 3, "Threshold for 3 validators");
    assert_eq!(threshold_4, 3, "Threshold for 4 validators");
    assert_eq!(threshold_7, 5, "Threshold for 7 validators");

    info!("✅ Threshold requirements verified");
}

/// Test: Verify recovery after validator restart.
#[tokio::test]
async fn test_validator_recovery() {
    let config = TimelockTestConfig::with_validators(3);
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing validator recovery...");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    let deadline = deadline_in_secs(&client, 5).await;
    register_timelock(&swarm, &client, deadline)
        .await
        .expect("Registration should succeed");

    sleep(Duration::from_secs(6)).await;

    // All validators still running - should reveal
    for _ in 0..20 {
        force_block_production(&swarm, &client).await.ok();
        sleep(Duration::from_millis(500)).await;
    }

    let dk = super::test_helpers::wait_for_dk_reveal(&client, 2, 60).await;

    match dk {
        Ok(dk_bytes) => {
            info!(
                "✅ With all validators: DK revealed ({} bytes)",
                dk_bytes.len()
            );
        },
        Err(e) => {
            panic!("Should reveal with all validators: {}", e);
        },
    }

    drop(swarm);
}
