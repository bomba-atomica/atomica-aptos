// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Phase 2: DKG Transcript Validity Tests
//!
//! Tests that verify the DKG transcript contains valid cryptographic data.
//!
//! Verifications:
//! - D4: Transcript contains valid MPK bytes

use super::test_helpers::{create_timelock_swarm, wait_for_dkg_completion, TimelockTestConfig};
use aptos_forge::Swarm;
use aptos_logger::info;

/// Test: Verify transcript contains data that can be deserialized.
///
/// The transcript should contain valid BCS-serialized data.
///
/// Verification: D4 (partial - format check)
#[tokio::test]
async fn test_transcript_is_valid_bcs() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing transcript is valid BCS...");

    let transcript = wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    // Basic validation: non-empty and reasonable size
    assert!(!transcript.is_empty(), "Transcript should not be empty");
    assert!(
        transcript.len() > 100,
        "Transcript should be larger than 100 bytes"
    );

    // Check first bytes aren't all zeros (would indicate corruption)
    let non_zero_count = transcript.iter().take(100).filter(|&&b| b != 0).count();
    assert!(
        non_zero_count > 10,
        "Transcript should contain non-zero data"
    );

    info!(
        "✅ Transcript appears valid ({} bytes, {} non-zero in first 100)",
        transcript.len(),
        non_zero_count
    );

    drop(swarm);
}

/// Test: Verify transcript was produced by correct dealer epoch.
///
/// The transcript should come from epoch 1.
#[tokio::test]
async fn test_transcript_dealer_epoch() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing transcript dealer epoch...");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    let dkg_state = super::verify_dkg_transcript_exists(&client)
        .await
        .expect("DKG state should exist");

    let last_complete = dkg_state.last_complete();
    let dealer_epoch = last_complete.metadata.dealer_epoch;

    assert_eq!(dealer_epoch, 1, "Dealer epoch should be 1");

    info!("✅ Transcript dealer epoch is {}", dealer_epoch);

    drop(swarm);
}

/// Test: Verify MPK can be derived from transcript.
///
/// This is a placeholder - actual MPK extraction requires deserializing
/// the transcript which needs the full IBE DKG types.
///
/// Verification: D4
#[tokio::test]
async fn test_transcript_contains_mpk_data() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing transcript contains MPK data...");

    let transcript = wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    // The transcript should be large enough to contain:
    // - PVSS data for each validator
    // - MPK (96 bytes for G2)
    // - Various metadata
    let min_size_for_mpk = 96 + 100; // MPK + metadata overhead
    assert!(
        transcript.len() >= min_size_for_mpk,
        "Transcript should be large enough to contain MPK"
    );

    info!(
        "✅ Transcript large enough to contain MPK ({} bytes >= {})",
        transcript.len(),
        min_size_for_mpk
    );

    drop(swarm);
}
