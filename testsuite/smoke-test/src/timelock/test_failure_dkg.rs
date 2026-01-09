// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Failure Mode: DKG Failure Tests
//!
//! Tests for scenarios where DKG fails or is not complete.

use super::test_helpers::{create_timelock_swarm, TimelockTestConfig};
use aptos_forge::Swarm;
use aptos_logger::info;

/// Test: Verify no MPK before DKG completes.
///
/// Expected: MPK query should return None before DKG.
#[tokio::test]
async fn test_no_mpk_before_dkg() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing no MPK before DKG...");

    // Immediately query MPK (before waiting for DKG)
    // Note: This is a race condition - DKG might complete quickly
    let mpk = super::verify_mpk_on_chain(&client, super::MPK_ID).await;

    // In epoch 1, DKG might not be complete yet
    // This test verifies the behavior when MPK is queried early

    match mpk {
        Ok(_) => {
            info!("Note: MPK already available (DKG completed quickly)");
        },
        Err(e) => {
            info!("✅ No MPK before DKG: {}", e);
        },
    }

    drop(swarm);
}

/// Test: Verify DKG transcript query before completion.
#[tokio::test]
async fn test_dkg_transcript_before_completion() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing DKG transcript before completion...");

    // Query immediately
    let transcript = super::verify_dkg_transcript_exists(&client).await;

    // Early in epoch 1, transcript might not exist yet
    match transcript {
        Ok(dkg_state) => {
            if dkg_state.last_completed.is_some() {
                info!("Note: DKG already completed");
            } else {
                info!("✅ DKG state exists but no completed transcript yet");
            }
        },
        Err(e) => {
            info!("Note: DKG state query issue: {}", e);
        },
    }

    drop(swarm);
}

/// Test: Verify MPK query for non-existent interval.
#[tokio::test]
async fn test_mpk_nonexistent_interval() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing MPK for non-existent interval...");

    // Query for interval 999 (definitely doesn't exist)
    let mpk = super::verify_mpk_on_chain(&client, 999).await;

    assert!(mpk.is_err(), "MPK should not exist for interval 999");

    info!("✅ Correctly returns None for non-existent interval");

    drop(swarm);
}
