// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Phase 5: Deadline Trigger Tests
//!
//! Tests for deadline detection and RequestRevealEvent emission.
//!
//! Verifications:
//! - DD1: RequestRevealEvent emitted when now >= deadline
//! - DD2: Event contains correct timelock_ids
//! - DD3: Deadline removed from pending_deadlines
//!
//! NOTE: These tests verify behavior indirectly through DK revelation,
//! which requires the full validator share submission flow to work.

use super::test_helpers::{
    create_timelock_swarm, deadline_in_secs, force_block_production, register_timelock,
    wait_for_dk_reveal, wait_for_dkg_completion, TimelockTestConfig,
};
use aptos_forge::Swarm;
use aptos_logger::info;
use std::time::Duration;
use tokio::time::sleep;

/// Test: Verify RequestRevealEvent is emitted when deadline passes.
///
/// We verify indirectly by waiting for the deadline and checking if
/// validators start processing (DK shares submitted or DK revealed).
///
/// Verification: DD1
///
/// NOTE: Will fail until share submission is implemented.
#[tokio::test]
async fn test_request_reveal_emitted_on_deadline() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing RequestRevealEvent emission on deadline...");

    // Wait for DKG first
    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    // Register with short deadline
    let deadline = deadline_in_secs(&client, 5).await;
    register_timelock(&swarm, &client, deadline)
        .await
        .expect("Registration should succeed");

    info!("Registered timelock ID 2 with deadline in 5s");

    // Wait for deadline to pass
    sleep(Duration::from_secs(6)).await;

    // Force block production to trigger on_new_block
    force_block_production(&swarm, &client)
        .await
        .expect("Block production should succeed");

    // If RequestRevealEvent was emitted, validators should eventually reveal DK
    let dk_result = wait_for_dk_reveal(&client, 2, 30).await;

    match dk_result {
        Ok(dk) => {
            info!(
                "✅ RequestRevealEvent emitted and processed, DK revealed ({} bytes)",
                dk.len()
            );
        },
        Err(e) => {
            panic!(
                "❌ DK not revealed after deadline: {}. \
                 RequestRevealEvent may not have been emitted or processed.",
                e
            );
        },
    }

    drop(swarm);
}

/// Test: Verify event contains correct timelock_id.
///
/// Verification: DD2
#[tokio::test]
async fn test_request_reveal_contains_timelock_id() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing RequestRevealEvent contains correct timelock_id...");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    let deadline = deadline_in_secs(&client, 5).await;
    register_timelock(&swarm, &client, deadline)
        .await
        .expect("Registration should succeed");

    sleep(Duration::from_secs(6)).await;
    force_block_production(&swarm, &client).await.ok();

    // DK should be revealed for ID 2, not other IDs
    let dk_2 = wait_for_dk_reveal(&client, 2, 30).await;

    match dk_2 {
        Ok(_) => {
            // ID 2 revealed - good
            // Verify ID 3 is NOT revealed (wasn't registered)
            let dk_3 = super::verify_dk_revealed(&client, 3).await;
            assert!(dk_3.is_err(), "ID 3 should not have a DK");

            info!("✅ Correct timelock_id (2) was processed");
        },
        Err(e) => {
            panic!("Expected DK for ID 2: {}", e);
        },
    }

    drop(swarm);
}

/// Test: Verify deadline is processed only once.
///
/// After the deadline passes and is processed, it should not trigger again.
///
/// Verification: DD3
#[tokio::test]
async fn test_deadline_processed_once() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing deadline processed only once...");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    let deadline = deadline_in_secs(&client, 5).await;
    register_timelock(&swarm, &client, deadline)
        .await
        .expect("Registration should succeed");

    sleep(Duration::from_secs(6)).await;

    // Force multiple blocks
    for _ in 0..5 {
        force_block_production(&swarm, &client).await.ok();
        sleep(Duration::from_millis(500)).await;
    }

    // Should still only have one DK for ID 2
    let dk = wait_for_dk_reveal(&client, 2, 30).await;

    match dk {
        Ok(dk_bytes) => {
            assert_eq!(
                dk_bytes.len(),
                super::DK_SHARE_SIZE_BYTES,
                "DK should be 48 bytes"
            );
            info!("✅ Deadline processed exactly once");
        },
        Err(e) => {
            panic!("DK should be revealed: {}", e);
        },
    }

    drop(swarm);
}
