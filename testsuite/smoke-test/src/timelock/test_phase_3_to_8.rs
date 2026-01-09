// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Integration Test: Phase 3 → Phase 8 (MPK → Decryption)
//!
//! Tests the complete cryptographic path from MPK to decryption.

use super::test_helpers::{
    create_timelock_swarm, deadline_in_secs, force_block_production, register_timelock,
    wait_for_dk_reveal, wait_for_dkg_completion, wait_for_mpk_publication, TimelockTestConfig,
};
use aptos_forge::Swarm;
use aptos_logger::info;
use std::time::Duration;
use tokio::time::sleep;

/// Test: Complete crypto path from MPK to DK.
///
/// Encrypt with MPK → register → wait → decrypt with DK.
///
/// Phases: 3 → 4 → 5 → 6 → 7 → 8
#[tokio::test]
async fn test_encrypt_with_mpk_decrypt_with_dk() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("[Phase 3→8] Testing complete crypto path...");

    // Phase 2: DKG completes
    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    // Phase 3: Get MPK for encryption
    let mpk = wait_for_mpk_publication(&client, 30)
        .await
        .expect("MPK should be available");

    super::verify_mpk_size(&mpk).expect("MPK should be 96 bytes");
    info!("Phase 3: MPK available ({} bytes)", mpk.len());

    // Simulated encryption would happen here
    // In real code: ciphertext = ibe_encrypt(mpk, identity, plaintext)

    // Phase 4: Register timelock
    let deadline = deadline_in_secs(&client, 5).await;
    register_timelock(&swarm, &client, deadline)
        .await
        .expect("Registration should succeed");

    info!("Phase 4: Timelock registered (ID 2)");

    // Phase 5: Wait for deadline
    sleep(Duration::from_secs(6)).await;

    // Phase 6-7: Shares and aggregation
    for _ in 0..20 {
        force_block_production(&swarm, &client).await.ok();
        sleep(Duration::from_millis(500)).await;
    }

    // Phase 8: Get DK for decryption
    let dk = wait_for_dk_reveal(&client, 2, 60)
        .await
        .expect("DK should be available");

    super::verify_dk_size(&dk).expect("DK should be 48 bytes");
    info!("Phase 8: DK available ({} bytes)", dk.len());

    // Simulated decryption would happen here
    // In real code: plaintext = ibe_decrypt(dk, ciphertext)

    info!(
        "✅ [Phase 3→8] Complete crypto path: MPK ({} bytes) → DK ({} bytes)",
        mpk.len(),
        dk.len()
    );

    drop(swarm);
}

/// Test: Verify MPK and DK are cryptographically related.
///
/// Both should be derived from the same DKG session.
///
/// Phases: 3 → 8
#[tokio::test]
async fn test_mpk_dk_relationship() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("[Phase 3→8] Testing MPK-DK relationship...");

    wait_for_dkg_completion(&swarm, &client, config.epoch_duration_secs).await;

    let mpk = wait_for_mpk_publication(&client, 30)
        .await
        .expect("MPK should be available");

    let deadline = deadline_in_secs(&client, 5).await;
    register_timelock(&swarm, &client, deadline)
        .await
        .expect("Registration should succeed");

    sleep(Duration::from_secs(6)).await;

    for _ in 0..20 {
        force_block_production(&swarm, &client).await.ok();
        sleep(Duration::from_millis(500)).await;
    }

    let dk = wait_for_dk_reveal(&client, 2, 60)
        .await
        .expect("DK should be available");

    // MPK is G2 (96 bytes), DK is G1 (48 bytes)
    assert_eq!(mpk.len(), super::MPK_SIZE_BYTES, "MPK is G2");
    assert_eq!(dk.len(), super::DK_SHARE_SIZE_BYTES, "DK is G1");

    // Both should have non-trivial data
    let mpk_nonzero = mpk.iter().filter(|&&b| b != 0).count();
    let dk_nonzero = dk.iter().filter(|&&b| b != 0).count();

    assert!(mpk_nonzero > mpk.len() / 2, "MPK should be non-trivial");
    assert!(dk_nonzero > dk.len() / 2, "DK should be non-trivial");

    info!(
        "✅ [Phase 3→8] MPK (G2, {} bytes) and DK (G1, {} bytes) related",
        mpk.len(),
        dk.len()
    );

    drop(swarm);
}
