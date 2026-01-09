// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Phase 7: Aggregation Tests
//!
//! Tests for Lagrange aggregation correctness.
//!
//! Verifications:
//! - A1: Aggregated DK stored in decryption_keys[id]
//! - A2: Aggregated DK is 48 bytes (compressed G1)
//! - A3: SecretRevealedEvent emitted
//! - A4: Event contains correct DK bytes

use super::test_helpers::{
    create_timelock_swarm, deadline_in_secs, force_block_production, register_timelock,
    wait_for_dk_reveal, wait_for_dkg_completion, TimelockTestConfig,
};
use aptos_forge::Swarm;
use aptos_logger::info;
use std::time::Duration;
use tokio::time::sleep;

/// Test: Verify aggregated DK is stored.
///
/// Verification: A1
#[tokio::test]
async fn test_aggregated_dk_stored() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing aggregated DK stored...");

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
        .expect("DK should be stored after aggregation");

    assert!(!dk.is_empty(), "DK should not be empty");

    info!("✅ Aggregated DK stored ({} bytes)", dk.len());

    drop(swarm);
}

/// Test: Verify aggregated DK is 48 bytes.
///
/// Verification: A2
#[tokio::test]
async fn test_aggregated_dk_is_48_bytes() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing aggregated DK is 48 bytes...");

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
        .expect("DK should be revealed");

    super::verify_dk_size(&dk).expect("DK should be 48 bytes");

    info!("✅ Aggregated DK is {} bytes", dk.len());

    drop(swarm);
}

/// Test: Verify SecretRevealedEvent is emitted (indirectly).
///
/// We verify by confirming DK is available, which only happens
/// after the event is emitted.
///
/// Verification: A3
#[tokio::test]
async fn test_secret_revealed_event_emitted() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing SecretRevealedEvent emission...");

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
        .expect("DK availability implies SecretRevealedEvent was emitted");

    assert!(!dk.is_empty());

    info!("✅ SecretRevealedEvent emitted (DK available)");

    drop(swarm);
}

/// Test: Verify aggregated DK is consistent.
///
/// Multiple queries should return the same DK.
///
/// Verification: A4 (consistency)
#[tokio::test]
async fn test_aggregated_dk_consistent() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing aggregated DK consistency...");

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

    // Query DK multiple times
    let dk1 = wait_for_dk_reveal(&client, 2, 30)
        .await
        .expect("DK should be revealed");
    let dk2 = super::verify_dk_revealed(&client, 2)
        .await
        .expect("DK should still be available");
    let dk3 = super::verify_dk_revealed(&client, 2)
        .await
        .expect("DK should still be available");

    assert_eq!(dk1, dk2, "DK should be consistent across queries");
    assert_eq!(dk2, dk3, "DK should be consistent across queries");

    info!("✅ Aggregated DK is consistent");

    drop(swarm);
}

/// Test: Verify aggregation produces valid cryptographic output.
///
/// The DK should have high entropy (not all zeros or patterns).
#[tokio::test]
async fn test_aggregation_produces_valid_output() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing aggregation produces valid output...");

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
        .expect("DK should be revealed");

    // Check entropy: count unique bytes
    let mut byte_counts = [0u8; 256];
    for &b in &dk {
        byte_counts[b as usize] = 1;
    }
    let unique_bytes: usize = byte_counts.iter().map(|&c| c as usize).sum();

    // A valid G1 point should have reasonable entropy
    assert!(
        unique_bytes >= 10,
        "DK should have at least 10 unique byte values, got {}",
        unique_bytes
    );

    info!(
        "✅ Aggregation produces valid output ({} unique bytes)",
        unique_bytes
    );

    drop(swarm);
}
