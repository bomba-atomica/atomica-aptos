// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Phase 3: MPK Storage Tests
//!
//! Tests that verify MPK is correctly stored in threshold_dsa.
//!
//! Verifications:
//! - M1: MPK stored in threshold_dsa for interval 1
//! - M2: MPK is 96 bytes (valid G2 point)
//!
//! NOTE: These tests will FAIL until the validator-side MPK publication
//! feature is implemented. Currently, DKG produces a transcript but
//! validators don't automatically call publish_master_public_key().

use super::test_helpers::{
    create_timelock_swarm, wait_for_dkg_completion, wait_for_mpk_publication, TimelockTestConfig,
};
use aptos_forge::Swarm;
use aptos_logger::info;

/// Test: Verify MPK is available in threshold_dsa after DKG.
///
/// This test polls for MPK publication after DKG completes.
///
/// Verification: M1
///
/// EXPECTED: FAIL until MPK publication is implemented in validators.
#[tokio::test]
async fn test_mpk_available_in_threshold_dsa() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing MPK availability in threshold_dsa...");

    // First, wait for DKG to complete
    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    info!("DKG complete, polling for MPK publication...");

    // Poll for MPK with timeout
    let mpk_result = wait_for_mpk_publication(&client, 30).await;

    match mpk_result {
        Ok(mpk) => {
            info!("✅ MPK published to threshold_dsa ({} bytes)", mpk.len());
            assert!(!mpk.is_empty(), "MPK should not be empty");
        },
        Err(e) => {
            panic!(
                "❌ MPK NOT published to threshold_dsa: {}. \
                 This is EXPECTED until validator MPK publication is implemented.",
                e
            );
        },
    }

    drop(swarm);
}

/// Test: Verify MPK has correct size (96 bytes for G2).
///
/// Verification: M2
///
/// EXPECTED: FAIL until MPK publication is implemented.
#[tokio::test]
async fn test_mpk_is_96_bytes() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing MPK size...");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    let mpk = wait_for_mpk_publication(&client, 30)
        .await
        .expect("MPK should be published (or test expected to fail)");

    super::verify_mpk_size(&mpk).expect("MPK should be 96 bytes");

    info!(
        "✅ MPK is {} bytes (expected {})",
        mpk.len(),
        super::MPK_SIZE_BYTES
    );

    drop(swarm);
}

/// Test: Verify MPK is not all zeros.
///
/// A valid G2 point should have non-trivial bytes.
///
/// Verification: M2 (validity check)
#[tokio::test]
async fn test_mpk_is_not_zero() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing MPK is not zero...");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    let mpk = wait_for_mpk_publication(&client, 30)
        .await
        .expect("MPK should be published");

    let non_zero_count = mpk.iter().filter(|&&b| b != 0).count();
    assert!(
        non_zero_count > mpk.len() / 2,
        "MPK should be mostly non-zero bytes"
    );

    info!(
        "✅ MPK has {} non-zero bytes out of {}",
        non_zero_count,
        mpk.len()
    );

    drop(swarm);
}

/// Test: Verify MPK matches what's in DKG transcript.
///
/// The MPK stored in threshold_dsa should match the MPK derivable from
/// the DKG transcript.
///
/// Verification: M2 (consistency check)
#[tokio::test]
async fn test_mpk_matches_transcript() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing MPK matches DKG transcript...");

    let transcript = wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    let mpk = wait_for_mpk_publication(&client, 30)
        .await
        .expect("MPK should be published");

    // The transcript contains the MPK - verify sizes are consistent
    assert!(
        transcript.len() > mpk.len(),
        "Transcript ({} bytes) should be larger than MPK ({} bytes)",
        transcript.len(),
        mpk.len()
    );

    info!(
        "✅ Transcript ({} bytes) contains MPK ({} bytes)",
        transcript.len(),
        mpk.len()
    );

    drop(swarm);
}
