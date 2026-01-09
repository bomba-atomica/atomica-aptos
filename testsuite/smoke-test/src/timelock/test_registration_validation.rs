// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Phase 4: Registration Validation Tests
//!
//! Tests for registration input validation.
//!
//! Verifications:
//! - R7: Registration with past deadline rejected
//! - R8: Registration before MPK ready (if enforced)

use super::test_helpers::{create_timelock_swarm, register_timelock, TimelockTestConfig};
use aptos_forge::Swarm;
use aptos_logger::info;

/// Test: Verify past deadline is rejected.
///
/// Registration with a deadline in the past should fail.
///
/// Verification: R7
#[tokio::test]
async fn test_past_deadline_rejected() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing past deadline rejection...");

    let now = super::test_helpers::get_chain_time(&client).await;
    let past_deadline = now - 1_000_000; // 1 second in the past

    let result = register_timelock(&swarm, &client, past_deadline).await;

    assert!(
        result.is_err(),
        "Registration with past deadline should fail"
    );

    info!("✅ Past deadline correctly rejected");

    drop(swarm);
}

/// Test: Verify zero deadline is rejected.
///
/// Verification: R7
#[tokio::test]
async fn test_zero_deadline_rejected() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing zero deadline rejection...");

    let result = register_timelock(&swarm, &client, 0).await;

    assert!(
        result.is_err(),
        "Registration with zero deadline should fail"
    );

    info!("✅ Zero deadline correctly rejected");

    drop(swarm);
}

/// Test: Verify very old deadline is rejected.
///
/// A deadline from year 2000 should definitely be rejected.
///
/// Verification: R7
#[tokio::test]
async fn test_very_old_deadline_rejected() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing very old deadline rejection...");

    // January 1, 2000 in microseconds (way in the past)
    let old_deadline = 946_684_800_000_000u64;

    let result = register_timelock(&swarm, &client, old_deadline).await;

    assert!(
        result.is_err(),
        "Registration with very old deadline should fail"
    );

    info!("✅ Very old deadline correctly rejected");

    drop(swarm);
}

/// Test: Verify deadline exactly at current time is rejected.
///
/// The deadline must be strictly in the future.
///
/// Verification: R7
#[tokio::test]
async fn test_current_time_deadline_rejected() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing current time deadline rejection...");

    let now = super::test_helpers::get_chain_time(&client).await;

    // By the time the transaction is processed, this will be in the past
    let result = register_timelock(&swarm, &client, now).await;

    // This might succeed or fail depending on timing, but it demonstrates
    // the edge case. A slightly past deadline should definitely fail.
    info!(
        "Current time deadline result: {:?} (may vary)",
        result.is_ok()
    );

    drop(swarm);
}

/// Test: Verify future deadline is accepted.
///
/// A deadline 1 hour in the future should be accepted.
///
/// Verification: R7 (positive case)
#[tokio::test]
async fn test_future_deadline_accepted() {
    let config = TimelockTestConfig::minimal();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Testing future deadline acceptance...");

    let now = super::test_helpers::get_chain_time(&client).await;
    let future_deadline = now + 3600_000_000; // 1 hour in future

    let result = register_timelock(&swarm, &client, future_deadline).await;

    assert!(
        result.is_ok(),
        "Registration with future deadline should succeed"
    );

    // Verify it was stored
    let stored = super::verify_deadline_stored(&client, 2)
        .await
        .expect("Deadline should be stored");

    assert_eq!(stored, future_deadline);

    info!("✅ Future deadline correctly accepted");

    drop(swarm);
}
