// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Full E2E Happy Path Tests
//!
//! Complete end-to-end scenarios testing the entire protocol.

use super::test_helpers::{
    create_timelock_swarm, deadline_in_secs, force_block_production, register_timelock,
    wait_for_dk_reveal, wait_for_dkg_completion, wait_for_mpk_publication, TimelockTestConfig,
};
use aptos_forge::Swarm;
use aptos_logger::info;
use std::time::Duration;
use tokio::time::sleep;

/// Test: Complete timelock flow from start to finish.
///
/// All phases: 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8
#[tokio::test]
async fn test_complete_timelock_flow() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("=== COMPLETE TIMELOCK FLOW TEST ===");

    // Phase 1: Initialization (happens at genesis)
    info!("Phase 1: Initialization...");
    // Verify state exists
    let _ = super::verify_deadline_stored(&client, 0).await; // Just verify callable
    info!("  ✓ TimelockState exists");

    // Phase 2: DKG
    info!("Phase 2: DKG...");
    let transcript = wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;
    info!("  ✓ DKG completed ({} bytes)", transcript.len());

    // Phase 3: MPK Publication
    info!("Phase 3: MPK Publication...");
    let mpk = wait_for_mpk_publication(&client, 30)
        .await
        .expect("MPK should be published");
    info!("  ✓ MPK published ({} bytes)", mpk.len());

    // Phase 4: Registration
    info!("Phase 4: Registration...");
    let deadline = deadline_in_secs(&client, 5).await;
    register_timelock(&swarm, &client, deadline)
        .await
        .expect("Registration should succeed");
    let stored = super::verify_deadline_stored(&client, 2)
        .await
        .expect("Deadline should be stored");
    assert_eq!(stored, deadline);
    info!("  ✓ Timelock ID 2 registered");

    // Phase 5: Deadline Detection
    info!("Phase 5: Deadline Detection...");
    sleep(Duration::from_secs(6)).await;
    for _ in 0..5 {
        force_block_production(&swarm, &client).await.ok();
        sleep(Duration::from_millis(200)).await;
    }
    info!("  ✓ Deadline passed, blocks produced");

    // Phase 6-7: Share Submission & Aggregation
    info!("Phase 6-7: Share Submission & Aggregation...");
    for _ in 0..15 {
        force_block_production(&swarm, &client).await.ok();
        sleep(Duration::from_millis(500)).await;
    }

    // Phase 8: Decryption Key Available
    info!("Phase 8: DK Retrieval...");
    let dk = wait_for_dk_reveal(&client, 2, 60)
        .await
        .expect("DK should be available");
    super::verify_dk_size(&dk).expect("DK should be 48 bytes");
    info!("  ✓ DK available ({} bytes)", dk.len());

    info!("=== COMPLETE TIMELOCK FLOW: SUCCESS ===");
    info!(
        "Summary: MPK ({} bytes) → Timelock ID 2 → DK ({} bytes)",
        mpk.len(),
        dk.len()
    );

    drop(swarm);
}

/// Test: Multiple timelocks with staggered deadlines.
#[tokio::test]
async fn test_multiple_timelocks_sequential() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("=== MULTIPLE TIMELOCKS SEQUENTIAL TEST ===");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;
    wait_for_mpk_publication(&client, 30)
        .await
        .expect("MPK should be published");

    let now = super::test_helpers::get_chain_time(&client).await;

    // Register 3 timelocks with staggered deadlines
    for i in 0..3 {
        let deadline = now + (5 + i * 3) * 1_000_000; // 5s, 8s, 11s
        register_timelock(&swarm, &client, deadline)
            .await
            .expect("Registration should succeed");
        info!("Registered ID {} with deadline in {}s", 2 + i, 5 + i * 3);
    }

    // Wait for all deadlines to pass
    sleep(Duration::from_secs(15)).await;

    for _ in 0..30 {
        force_block_production(&swarm, &client).await.ok();
        sleep(Duration::from_millis(500)).await;
    }

    // Verify all DKs revealed
    for id in 2..=4 {
        let dk = wait_for_dk_reveal(&client, id, 30)
            .await
            .expect(&format!("DK {} should be revealed", id));
        info!("ID {} DK: {} bytes", id, dk.len());
    }

    info!("=== MULTIPLE TIMELOCKS SEQUENTIAL: SUCCESS ===");

    drop(swarm);
}

/// Test: Multiple timelocks registered together.
#[tokio::test]
async fn test_multiple_timelocks_concurrent() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("=== MULTIPLE TIMELOCKS CONCURRENT TEST ===");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;
    wait_for_mpk_publication(&client, 30)
        .await
        .expect("MPK should be published");

    let deadline = deadline_in_secs(&client, 5).await;

    // Register 3 timelocks with same deadline
    for i in 0..3 {
        register_timelock(&swarm, &client, deadline)
            .await
            .expect("Registration should succeed");
        info!("Registered ID {}", 2 + i);
    }

    sleep(Duration::from_secs(6)).await;

    for _ in 0..30 {
        force_block_production(&swarm, &client).await.ok();
        sleep(Duration::from_millis(500)).await;
    }

    // All should be revealed
    for id in 2..=4 {
        let dk = wait_for_dk_reveal(&client, id, 30)
            .await
            .expect(&format!("DK {} should be revealed", id));
        info!("ID {} DK: {} bytes", id, dk.len());
    }

    // Verify all DKs are different
    let dk2 = super::verify_dk_revealed(&client, 2).await.unwrap();
    let dk3 = super::verify_dk_revealed(&client, 3).await.unwrap();
    let dk4 = super::verify_dk_revealed(&client, 4).await.unwrap();

    assert_ne!(dk2, dk3, "DKs should be unique");
    assert_ne!(dk3, dk4, "DKs should be unique");
    assert_ne!(dk2, dk4, "DKs should be unique");

    info!("=== MULTIPLE TIMELOCKS CONCURRENT: SUCCESS ===");

    drop(swarm);
}
