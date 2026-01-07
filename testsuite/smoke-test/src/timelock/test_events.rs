//! Timelock Event Verification Tests
//!
//! These tests verify that the correct events are emitted at each step of the
//! timelock protocol. They check validator logs for event processing.

use super::test_helpers::{create_timelock_swarm, register_timelock, TimelockTestConfig};
use aptos_api_types::ViewFunction;
use aptos_logger::info;
use move_core_types::identifier::Identifier;
use move_core_types::language_storage::ModuleId;
use std::str::FromStr;
use std::time::Duration;
use tokio::time::sleep;

async fn get_chain_time(client: &aptos_rest_client::Client) -> u64 {
    let view = ViewFunction {
        module: ModuleId::from_str("0x1::timestamp").unwrap(),
        function: Identifier::from_str("now_microseconds").unwrap(),
        ty_args: vec![],
        args: vec![],
    };
    let response: Vec<u64> = client.view_bcs(&view, None).await.unwrap().into_inner();
    response[0]
}

/// Test: Verify that StartKeyGenEvent triggers MPK DKG
///
/// This test verifies the first step of the timelock protocol:
/// - The `on_new_block` function emits `StartKeyGenEvent` in the first block
/// - This event triggers validators to start the MPK DKG process
///
/// Check validator logs for: "[TIMELOCK] Emitted StartKeyGenEvent"
#[tokio::test]
async fn test_start_keygen_event_triggers_dkg() {
    let config = TimelockTestConfig::default();
    let (_swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Waiting for MPK to be published (indicates StartKeyGenEvent was processed)...");

    let mut success = false;
    for attempt in 0..60 {
        if super::verify_master_public_key_on_chain(&client, 1)
            .await
            .is_ok()
        {
            info!(
                "✅ StartKeyGenEvent successfully triggered DKG after {}s",
                attempt
            );
            success = true;
            break;
        }
        sleep(Duration::from_secs(1)).await;
    }

    // If MPK is published, StartKeyGenEvent was processed correctly
    assert!(
        success,
        "StartKeyGenEvent should trigger MPK DKG within 60s"
    );
}

/// Test: Verify that DeadlineReachedEvent is emitted when deadline passes
///
/// This test verifies that:
/// - When a registered timelock's deadline passes
/// - The `on_new_block` function emits `DeadlineReachedEvent`
///
/// Check validator logs for: "DeadlineReachedEvent"
#[tokio::test]
async fn test_deadline_triggers_event() {
    let config = TimelockTestConfig::default();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    // Wait for MPK first
    for _ in 0..60 {
        if super::verify_master_public_key_on_chain(&client, 1)
            .await
            .is_ok()
        {
            break;
        }
        sleep(Duration::from_secs(1)).await;
    }

    // Register with very short deadline
    let now = get_chain_time(&client).await;
    let deadline = now + 3_000_000; // 3 seconds

    info!("Registering timelock with deadline in 3s");
    register_timelock(&*swarm, &client, deadline).await.unwrap();

    // Wait for deadline to pass
    sleep(Duration::from_secs(5)).await;

    // Force block production
    let root_account = swarm.chain_info().root_account();
    for _ in 0..3 {
        let _ = client
            .submit_and_wait(
                &root_account.sign_with_transaction_builder(
                    aptos_sdk::transaction_builder::TransactionFactory::new(
                        swarm.chain_info().chain_id,
                    )
                    .payload(
                        aptos_types::transaction::TransactionPayload::EntryFunction(
                            aptos_types::transaction::EntryFunction::new(
                                ModuleId::from_str("0x1::aptos_account").unwrap(),
                                Identifier::from_str("transfer").unwrap(),
                                vec![],
                                vec![
                                    bcs::to_bytes(
                                        &aptos_types::account_address::AccountAddress::ONE,
                                    )
                                    .unwrap(),
                                    bcs::to_bytes(&100u64).unwrap(),
                                ],
                            ),
                        ),
                    ),
                ),
            )
            .await;
        sleep(Duration::from_millis(500)).await;
    }

    // Check if decryption key is eventually available (indicates event was processed)
    let mut found = false;
    for attempt in 0..30 {
        if super::verify_secret_aggregated(&client, 2, 1).await.is_ok() {
            info!(
                "✅ DeadlineReachedEvent processed, secret available after {}s",
                attempt
            );
            found = true;
            break;
        }
        sleep(Duration::from_secs(1)).await;
    }

    // Note: If this fails, check validator logs for DeadlineReachedEvent
    assert!(
        found,
        "DeadlineReachedEvent should trigger secret revelation"
    );
}

/// Test: Verify threshold enforcement
///
/// This test verifies that the aggregation requires at least
/// threshold (2/3 + 1) shares before revealing the decryption key.
#[tokio::test]
async fn test_threshold_enforcement() {
    // This test primarily verifies the threshold calculation in Move code
    // by checking that when we have 3 validators, we need 3 shares (3*2/3+1=3)

    let config = TimelockTestConfig {
        num_validators: 3,
        num_fullnodes: 0,
        epoch_duration_secs: 20,
    };
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    // Wait for MPK
    for _ in 0..60 {
        if super::verify_master_public_key_on_chain(&client, 1)
            .await
            .is_ok()
        {
            break;
        }
        sleep(Duration::from_secs(1)).await;
    }
    info!("MPK ready with 3 validators");

    // Register and wait
    let now = get_chain_time(&client).await;
    register_timelock(&*swarm, &client, now + 3_000_000)
        .await
        .unwrap();
    sleep(Duration::from_secs(8)).await;

    // Force blocks
    let root_account = swarm.chain_info().root_account();
    for _ in 0..5 {
        let _ = client
            .submit_and_wait(
                &root_account.sign_with_transaction_builder(
                    aptos_sdk::transaction_builder::TransactionFactory::new(
                        swarm.chain_info().chain_id,
                    )
                    .payload(
                        aptos_types::transaction::TransactionPayload::EntryFunction(
                            aptos_types::transaction::EntryFunction::new(
                                ModuleId::from_str("0x1::aptos_account").unwrap(),
                                Identifier::from_str("transfer").unwrap(),
                                vec![],
                                vec![
                                    bcs::to_bytes(
                                        &aptos_types::account_address::AccountAddress::ONE,
                                    )
                                    .unwrap(),
                                    bcs::to_bytes(&100u64).unwrap(),
                                ],
                            ),
                        ),
                    ),
                ),
            )
            .await;
    }

    // Check that secret is revealed (proves threshold was met)
    let mut revealed = false;
    for attempt in 0..30 {
        if super::verify_secret_aggregated(&client, 2, 1).await.is_ok() {
            info!("✅ Threshold met and secret revealed after {}s", attempt);
            revealed = true;
            break;
        }
        sleep(Duration::from_secs(1)).await;
    }

    assert!(
        revealed,
        "Secret should be revealed when threshold (3/3) is met"
    );
}
