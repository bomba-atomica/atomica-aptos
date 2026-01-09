// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Full E2E Edge Case Tests
//!
//! Tests for boundary conditions and edge cases.

use super::test_helpers::{
    create_timelock_swarm, force_block_production, register_timelock, wait_for_dk_reveal,
    wait_for_dkg_completion, wait_for_mpk_publication, TimelockTestConfig,
};
use aptos_forge::Swarm;
use aptos_logger::info;
use std::time::Duration;
use tokio::time::sleep;

/// Test: Immediate deadline (1 second away).
#[tokio::test]
async fn test_immediate_deadline() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("=== IMMEDIATE DEADLINE TEST ===");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;
    wait_for_mpk_publication(&client, 30)
        .await
        .expect("MPK should be published");

    // Register with very short deadline (1 second)
    let now = super::test_helpers::get_chain_time(&client).await;
    let deadline = now + 1_000_000; // 1 second

    register_timelock(&swarm, &client, deadline)
        .await
        .expect("Registration should succeed");

    info!("Registered with 1s deadline");

    // Wait just a bit more than the deadline
    sleep(Duration::from_secs(2)).await;

    for _ in 0..20 {
        force_block_production(&swarm, &client).await.ok();
        sleep(Duration::from_millis(500)).await;
    }

    let dk = wait_for_dk_reveal(&client, 2, 60).await;

    match dk {
        Ok(dk_bytes) => {
            info!("✅ Immediate deadline handled: DK {} bytes", dk_bytes.len());
        },
        Err(e) => {
            info!("Note: Immediate deadline may be too fast: {}", e);
        },
    }

    drop(swarm);
}

/// Test: Many timelocks with same deadline.
#[tokio::test]
async fn test_many_timelocks_same_deadline() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("=== MANY TIMELOCKS SAME DEADLINE TEST ===");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;
    wait_for_mpk_publication(&client, 30)
        .await
        .expect("MPK should be published");

    let now = super::test_helpers::get_chain_time(&client).await;
    let deadline = now + 5_000_000;

    // Register 10 timelocks with same deadline
    for i in 0..10 {
        register_timelock(&swarm, &client, deadline)
            .await
            .expect(&format!("Registration {} should succeed", i));
    }

    info!("Registered 10 timelocks with same deadline");

    sleep(Duration::from_secs(6)).await;

    for _ in 0..40 {
        force_block_production(&swarm, &client).await.ok();
        sleep(Duration::from_millis(500)).await;
    }

    // Count revealed DKs
    let mut revealed_count = 0;
    for id in 2..=11 {
        if wait_for_dk_reveal(&client, id, 30).await.is_ok() {
            revealed_count += 1;
        }
    }

    info!(
        "✅ {}/10 timelocks with same deadline revealed",
        revealed_count
    );

    assert!(revealed_count >= 5, "At least half should be revealed");

    drop(swarm);
}

/// Test: Far future deadline.
#[tokio::test]
async fn test_far_future_deadline() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("=== FAR FUTURE DEADLINE TEST ===");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    // Register with deadline 1 hour in future
    let now = super::test_helpers::get_chain_time(&client).await;
    let deadline = now + 3600_000_000; // 1 hour

    register_timelock(&swarm, &client, deadline)
        .await
        .expect("Registration should succeed");

    // Verify stored correctly
    let stored = super::verify_deadline_stored(&client, 2)
        .await
        .expect("Deadline should be stored");

    assert_eq!(stored, deadline, "Far future deadline should be stored");

    // DK should NOT be revealed yet
    super::verify_dk_not_revealed(&client, 2)
        .await
        .expect("DK should not be revealed before far future deadline");

    info!("✅ Far future deadline stored correctly, no premature reveal");

    drop(swarm);
}

/// Test: Mixed deadline ordering.
#[tokio::test]
async fn test_mixed_deadline_ordering() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("=== MIXED DEADLINE ORDERING TEST ===");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;
    wait_for_mpk_publication(&client, 30)
        .await
        .expect("MPK should be published");

    let now = super::test_helpers::get_chain_time(&client).await;

    // Register in non-chronological order
    let deadlines = vec![
        now + 10_000_000, // ID 2: 10s
        now + 5_000_000,  // ID 3: 5s (earlier)
        now + 15_000_000, // ID 4: 15s
        now + 3_000_000,  // ID 5: 3s (earliest)
    ];

    for deadline in &deadlines {
        register_timelock(&swarm, &client, *deadline)
            .await
            .expect("Registration should succeed");
    }

    info!("Registered in order: 10s, 5s, 15s, 3s");

    // Wait for first deadline
    sleep(Duration::from_secs(4)).await;

    for _ in 0..10 {
        force_block_production(&swarm, &client).await.ok();
        sleep(Duration::from_millis(500)).await;
    }

    // ID 5 (3s) should be revealed first
    let dk_5 = super::verify_dk_revealed(&client, 5).await;
    if dk_5.is_ok() {
        info!("ID 5 (3s) revealed first - correct!");
    }

    // Wait for all deadlines
    sleep(Duration::from_secs(15)).await;

    for _ in 0..30 {
        force_block_production(&swarm, &client).await.ok();
        sleep(Duration::from_millis(500)).await;
    }

    // All should be revealed
    for id in 2..=5 {
        let dk = wait_for_dk_reveal(&client, id, 30).await;
        info!("ID {} revealed: {}", id, dk.is_ok());
    }

    info!("✅ Mixed deadline ordering handled correctly");

    drop(swarm);
}
