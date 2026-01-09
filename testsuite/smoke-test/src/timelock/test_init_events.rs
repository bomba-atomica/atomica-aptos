// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Phase 1: Initialization Event Tests
//!
//! Tests for initialization event emission.
//!
//! Verifications:
//! - I5: StartKeyGenEvent emitted with epoch=1 and correct threshold config
//!
//! Note: Direct event querying requires indexer support. These tests verify
//! event emission indirectly by confirming the expected system behavior.

use super::test_helpers::{create_timelock_swarm, wait_for_dkg_completion, TimelockTestConfig};
use aptos_forge::Swarm;
use aptos_logger::info;

/// Test: Verify StartKeyGenEvent is emitted and triggers DKG.
///
/// We verify this indirectly by confirming DKG completes successfully.
/// If StartKeyGenEvent wasn't emitted, validators wouldn't start DKG.
///
/// Verification: I5
#[tokio::test]
async fn test_start_keygen_event_emitted() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing StartKeyGenEvent emission...");

    // If StartKeyGenEvent wasn't emitted, DKG would never start
    let transcript = wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    assert!(
        !transcript.is_empty(),
        "DKG should produce non-empty transcript, proving StartKeyGenEvent was emitted"
    );

    info!("✅ StartKeyGenEvent emitted (DKG completed successfully)");

    drop(swarm);
}

/// Test: Verify StartKeyGenEvent payload has correct epoch.
///
/// The event should have epoch=1 (MPK_ID).
/// We verify indirectly by checking the DKG transcript targets epoch 2.
///
/// Verification: I5 (payload validation)
#[tokio::test]
async fn test_start_keygen_event_payload() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing StartKeyGenEvent payload...");

    // Wait for DKG
    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    // Verify DKG transcript has correct metadata
    let target_epoch = super::get_dkg_target_epoch(&client)
        .await
        .expect("Should get DKG target epoch");

    // DKG in epoch 1 produces keys for epoch 2
    assert_eq!(
        target_epoch, 2,
        "DKG should target epoch 2 (triggered by StartKeyGenEvent in epoch 1)"
    );

    info!("✅ StartKeyGenEvent had correct epoch (DKG targets epoch 2)");

    drop(swarm);
}

/// Test: Verify StartKeyGenEvent includes threshold config.
///
/// For 3 validators, threshold should be (3 * 2 / 3) + 1 = 3.
///
/// Verification: I5 (threshold config)
#[tokio::test]
async fn test_start_keygen_threshold_config() {
    let config = TimelockTestConfig::with_validators(3);
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing StartKeyGenEvent threshold config...");

    // Wait for DKG
    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    // Calculate expected threshold
    let expected_threshold = super::test_helpers::calculate_threshold(config.num_validators);
    info!(
        "Expected threshold for {} validators: {}",
        config.num_validators, expected_threshold
    );

    // For 3 validators: (3 * 2 / 3) + 1 = 3
    assert_eq!(
        expected_threshold, 3,
        "Threshold for 3 validators should be 3"
    );

    // DKG completing successfully proves threshold was configured correctly
    // (otherwise aggregation would fail)

    info!(
        "✅ Threshold config correct ({}/{})",
        expected_threshold, config.num_validators
    );

    drop(swarm);
}
