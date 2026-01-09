// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Phase 2: DKG Completion Tests
//!
//! Tests that verify DKG completes successfully.
//!
//! Verifications:
//! - D1: DKG transcript produced by end of epoch 1
//! - D2: Transcript is non-empty
//! - D3: Transcript targets correct epoch (2)

use super::test_helpers::{create_timelock_swarm, wait_for_dkg_completion, TimelockTestConfig};
use aptos_forge::Swarm;
use aptos_logger::info;

/// Test: Verify DKG completes by epoch 2.
///
/// Verification: D1
#[tokio::test]
async fn test_dkg_completes_by_epoch_2() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing DKG completion by epoch 2...");

    // Wait for epoch 2
    super::test_helpers::wait_for_epoch(&swarm, 2, config.epoch_duration_secs * 3)
        .await
        .expect("Should reach epoch 2");

    // Verify DKG transcript exists
    let dkg_state = super::verify_dkg_transcript_exists(&client)
        .await
        .expect("DKG transcript should exist by epoch 2");

    assert!(
        dkg_state.last_completed.is_some(),
        "last_completed should be Some"
    );

    info!("✅ DKG completed by epoch 2");

    drop(swarm);
}

/// Test: Verify DKG transcript is non-empty.
///
/// Verification: D2
#[tokio::test]
async fn test_dkg_transcript_non_empty() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing DKG transcript is non-empty...");

    let transcript = wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    assert!(!transcript.is_empty(), "DKG transcript should be non-empty");

    info!(
        "✅ DKG transcript is non-empty ({} bytes)",
        transcript.len()
    );

    drop(swarm);
}

/// Test: Verify DKG transcript targets correct epoch.
///
/// DKG runs in epoch 1 and produces keys for epoch 2.
///
/// Verification: D3
#[tokio::test]
async fn test_dkg_transcript_target_epoch() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing DKG transcript target epoch...");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    let target_epoch = super::get_dkg_target_epoch(&client)
        .await
        .expect("Should get target epoch");

    assert_eq!(target_epoch, 2, "DKG should target epoch 2");

    info!("✅ DKG transcript targets epoch {}", target_epoch);

    drop(swarm);
}

/// Test: Verify DKG transcript has expected size range.
///
/// A valid transcript should be several KB (contains PVSS data).
#[tokio::test]
async fn test_dkg_transcript_size() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing DKG transcript size...");

    let transcript = wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    // Transcript should be at least 1KB for 3 validators
    let min_expected_size = 1000;
    assert!(
        transcript.len() >= min_expected_size,
        "Transcript should be at least {} bytes, got {}",
        min_expected_size,
        transcript.len()
    );

    info!(
        "✅ DKG transcript has reasonable size ({} bytes)",
        transcript.len()
    );

    drop(swarm);
}
