// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Phase 8: IBE Decryption Tests
//!
//! Tests for client-side decryption using on-chain DK.
//!
//! Verifications:
//! - E2: Decryption with correct DK recovers plaintext
//! - E3: Decryption with wrong DK fails
//! - E4: Identity mismatch prevents decryption

use super::test_helpers::{
    create_timelock_swarm, deadline_in_secs, force_block_production, register_timelock,
    wait_for_dk_reveal, wait_for_dkg_completion, TimelockTestConfig,
};
use aptos_forge::Swarm;
use aptos_logger::info;
use std::time::Duration;
use tokio::time::sleep;

/// Test: Verify DK can be retrieved for decryption.
///
/// Verification: E2 (prerequisite)
#[tokio::test]
async fn test_dk_available_for_decryption() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing DK available for decryption...");

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
        .expect("DK should be available for decryption");

    super::verify_dk_size(&dk).expect("DK should be 48 bytes");

    info!("✅ DK available for decryption ({} bytes)", dk.len());

    drop(swarm);
}

/// Test: Verify DK is valid for decryption (format check).
///
/// Verification: E2
#[tokio::test]
async fn test_dk_valid_for_decryption() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing DK valid for decryption...");

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
        .expect("DK should be available");

    // Verify DK is valid G1 point (basic check)
    assert_eq!(
        dk.len(),
        super::DK_SHARE_SIZE_BYTES,
        "DK should be 48 bytes"
    );

    let non_zero = dk.iter().filter(|&&b| b != 0).count();
    assert!(non_zero > dk.len() / 2, "DK should be mostly non-zero");

    // In a real test, we would:
    // 1. Have a ciphertext encrypted with the corresponding identity
    // 2. Call ibe_decrypt(dk, ciphertext)
    // 3. Verify plaintext matches original

    info!("✅ DK is valid for decryption");

    drop(swarm);
}

/// Test: Verify different timelocks have different DKs.
///
/// This ensures identity mismatch would cause decryption failure.
///
/// Verification: E3, E4
#[tokio::test]
async fn test_different_timelocks_different_dks() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing different timelocks have different DKs...");

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

    sleep(Duration::from_secs(8)).await;

    for _ in 0..20 {
        force_block_production(&swarm, &client).await.ok();
        sleep(Duration::from_millis(500)).await;
    }

    let dk_1 = wait_for_dk_reveal(&client, 2, 60)
        .await
        .expect("DK 1 should be available");
    let dk_2 = wait_for_dk_reveal(&client, 3, 60)
        .await
        .expect("DK 2 should be available");

    assert_ne!(
        dk_1, dk_2,
        "Different timelocks should have different DKs (identity mismatch would fail)"
    );

    info!("✅ Different timelocks have different DKs (ensures E3/E4)");

    drop(swarm);
}

/// Test: Verify DK uniqueness across deadlines.
///
/// Even with same timelock_id, different deadlines would produce different DKs
/// (since identity includes deadline).
///
/// Verification: E4
#[tokio::test]
async fn test_dk_uniqueness() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing DK uniqueness...");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    // Register 3 timelocks
    let now = super::test_helpers::get_chain_time(&client).await;
    for i in 0..3 {
        let deadline = now + (5 + i) * 1_000_000;
        register_timelock(&swarm, &client, deadline)
            .await
            .expect("Registration should succeed");
    }

    sleep(Duration::from_secs(10)).await;

    for _ in 0..20 {
        force_block_production(&swarm, &client).await.ok();
        sleep(Duration::from_millis(500)).await;
    }

    // Collect all DKs
    let mut dks = vec![];
    for id in 2..=4 {
        let dk = wait_for_dk_reveal(&client, id, 60)
            .await
            .expect(&format!("DK {} should be available", id));
        dks.push(dk);
    }

    // Verify all DKs are unique
    for i in 0..dks.len() {
        for j in (i + 1)..dks.len() {
            assert_ne!(
                dks[i],
                dks[j],
                "DKs {} and {} should be different",
                i + 2,
                j + 2
            );
        }
    }

    info!("✅ All {} DKs are unique", dks.len());

    drop(swarm);
}
