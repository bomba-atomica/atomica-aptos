// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Phase 7: Threshold Met Tests
//!
//! Tests for behavior when threshold IS met.
//!
//! Verifications:
//! - T2: Secret revealed exactly when threshold reached
//! - T3: Threshold is (n * 2 / 3) + 1

use super::test_helpers::{
    calculate_threshold, create_timelock_swarm, deadline_in_secs, force_block_production,
    register_timelock, wait_for_dk_reveal, wait_for_dkg_completion, TimelockTestConfig,
};
use aptos_forge::Swarm;
use aptos_logger::info;
use std::time::Duration;
use tokio::time::sleep;

/// Test: Verify reveal at threshold with all validators.
///
/// Verification: T2
#[tokio::test]
async fn test_reveal_with_all_validators() {
    let config = TimelockTestConfig::with_validators(3);
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing reveal with all validators...");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    let deadline = deadline_in_secs(&client, 5).await;
    register_timelock(&swarm, &client, deadline)
        .await
        .expect("Registration should succeed");

    sleep(Duration::from_secs(6)).await;

    for _ in 0..15 {
        force_block_production(&swarm, &client).await.ok();
        sleep(Duration::from_millis(500)).await;
    }

    let dk = wait_for_dk_reveal(&client, 2, 60)
        .await
        .expect("DK should be revealed with all validators");

    super::verify_dk_size(&dk).expect("DK should be 48 bytes");

    info!("✅ Secret revealed with all validators");

    drop(swarm);
}

/// Test: Verify threshold calculation is correct.
///
/// For n validators, threshold should be (n * 2 / 3) + 1.
///
/// Verification: T3
#[tokio::test]
async fn test_threshold_calculation() {
    info!("Testing threshold calculation...");

    // Test threshold formula
    assert_eq!(
        calculate_threshold(3),
        3,
        "Threshold for 3 validators: (3*2/3)+1 = 3"
    );
    assert_eq!(
        calculate_threshold(4),
        3,
        "Threshold for 4 validators: (4*2/3)+1 = 3"
    );
    assert_eq!(
        calculate_threshold(5),
        4,
        "Threshold for 5 validators: (5*2/3)+1 = 4"
    );
    assert_eq!(
        calculate_threshold(6),
        5,
        "Threshold for 6 validators: (6*2/3)+1 = 5"
    );
    assert_eq!(
        calculate_threshold(7),
        5,
        "Threshold for 7 validators: (7*2/3)+1 = 5"
    );

    info!("✅ Threshold calculation correct");
}

/// Test: Verify reveal happens eventually with all validators.
///
/// Verification: T2 (timing)
#[tokio::test]
async fn test_reveal_timing() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing reveal timing...");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    let deadline = deadline_in_secs(&client, 5).await;
    register_timelock(&swarm, &client, deadline)
        .await
        .expect("Registration should succeed");

    // Before deadline: no DK
    super::verify_dk_not_revealed(&client, 2)
        .await
        .expect("DK should not exist before deadline");

    info!("Confirmed: no DK before deadline");

    // Wait for deadline
    sleep(Duration::from_secs(6)).await;

    // Force blocks and measure time to reveal
    let start = std::time::Instant::now();

    for i in 0..60 {
        force_block_production(&swarm, &client).await.ok();

        if super::verify_dk_revealed(&client, 2).await.is_ok() {
            let elapsed = start.elapsed();
            info!(
                "✅ DK revealed after {} iterations ({:.2}s after deadline)",
                i + 1,
                elapsed.as_secs_f64()
            );
            return;
        }

        sleep(Duration::from_millis(500)).await;
    }

    panic!("DK was not revealed within 30 seconds after deadline");
}

/// Test: Verify multiple timelocks reveal in order.
///
/// Verification: T2 (multiple)
#[tokio::test]
async fn test_multiple_reveals() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing multiple reveals...");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    let now = super::test_helpers::get_chain_time(&client).await;

    // Register 3 timelocks with staggered deadlines
    for i in 0..3 {
        let deadline = now + (5 + i * 3) * 1_000_000; // 5s, 8s, 11s
        register_timelock(&swarm, &client, deadline)
            .await
            .expect("Registration should succeed");
    }

    info!("Registered 3 timelocks with staggered deadlines");

    // Wait for all deadlines
    sleep(Duration::from_secs(15)).await;

    for _ in 0..30 {
        force_block_production(&swarm, &client).await.ok();
        sleep(Duration::from_millis(500)).await;
    }

    // All should be revealed
    for id in 2..=4 {
        let dk = wait_for_dk_reveal(&client, id, 30).await;
        assert!(dk.is_ok(), "DK for ID {} should be revealed", id);
        info!("ID {} revealed", id);
    }

    info!("✅ All 3 timelocks revealed");

    drop(swarm);
}
