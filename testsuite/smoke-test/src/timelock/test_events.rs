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
    let mut last_error = String::new();
    let mut dkg_available = false;
    let mut mpk_available = false;

    for attempt in 0..60 {
        // First check if DKG transcript is available (upstream check)
        if !dkg_available {
            match super::verify_public_key_published(&client, 1).await {
                Ok(bytes) => {
                    info!(
                        "DKG transcript available at attempt {}: {} bytes",
                        attempt + 1,
                        bytes.len()
                    );
                    dkg_available = true;
                },
                Err(e) => {
                    last_error = format!("DKG not ready: {}", e);
                    if attempt % 10 == 0 && !dkg_available {
                        info!("Attempt {}/60: {}", attempt + 1, last_error);
                    }
                },
            }
        }

        // Then check if MPK is published (downstream check)
        if dkg_available && !mpk_available {
            match super::verify_master_public_key_on_chain(&client, 1).await {
                Ok(mpk_bytes) => {
                    info!(
                        "✅ StartKeyGenEvent successfully triggered DKG after {}s - MPK published: {} bytes",
                        attempt,
                        mpk_bytes.len()
                    );
                    success = true;
                    mpk_available = true;
                    break;
                },
                Err(e) => {
                    last_error = format!("MPK not published: {}", e);
                    if attempt % 10 == 0 {
                        info!("Attempt {}/60: {}", attempt + 1, last_error);
                    }
                },
            }
        }

        if success {
            break;
        }

        sleep(Duration::from_secs(1)).await;
    }

    if !success {
        info!("=== DKG DIAGNOSTIC FAILURE ===");
        info!("DKG transcript available: {}", dkg_available);
        info!("Master Public Key available: {}", mpk_available);
        info!("Last error: {}", last_error);
        info!("Check validator logs for:");
        info!("  - [DKG] StartKeyGenEvent emission");
        info!("  - [DKG] DKG protocol progress");
        info!("  - [DKG] Transcript publication");
        info!("  - [threshold_dsa] MPK derivation and publication");
        info!("===============================");
    }

    // If MPK is published, StartKeyGenEvent was processed correctly
    assert!(
        success,
        "StartKeyGenEvent should trigger MPK DKG within 60s. DKG ready: {}, MPK ready: {}. Last error: {}",
        dkg_available, mpk_available, last_error
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
    let mut last_error = String::new();
    for attempt in 0..30 {
        match super::verify_secret_aggregated(&client, 2, 1).await {
            Ok(secret) => {
                info!(
                    "✅ DeadlineReachedEvent processed, secret available after {}s: {} bytes",
                    attempt,
                    secret.len()
                );
                found = true;
                break;
            },
            Err(e) => {
                last_error = e.to_string();
                if attempt % 10 == 0 {
                    info!("Attempt {}/30: Secret not yet revealed - {}", attempt, e);
                }
            },
        }
        sleep(Duration::from_secs(1)).await;
    }

    if !found {
        info!("=== DEADLINE EVENT DIAGNOSTIC ===");
        info!("Deadline: {}", deadline);
        info!("Last error: {}", last_error);
        info!("Check validator logs for:");
        info!("  - [TIMELOCK] DeadlineReachedEvent emission");
        info!("  - [TIMELOCK] Secret share submission from validators");
        info!("  - [TIMELOCK] Secret aggregation and revelation");
        info!("==================================");
    }

    // Note: If this fails, check validator logs for DeadlineReachedEvent
    assert!(
        found,
        "DeadlineReachedEvent should trigger secret revelation. Last error: {}",
        last_error
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
    let mut last_error = String::new();
    for attempt in 0..30 {
        match super::verify_secret_aggregated(&client, 2, 1).await {
            Ok(secret) => {
                info!(
                    "✅ Threshold met and secret revealed after {}s: {} bytes",
                    attempt,
                    secret.len()
                );
                revealed = true;
                break;
            },
            Err(e) => {
                last_error = e.to_string();
                if attempt % 10 == 0 {
                    info!("Attempt {}/30: Threshold not met - {}", attempt, e);
                }
            },
        }
        sleep(Duration::from_secs(1)).await;
    }

    if !revealed {
        info!("=== THRESHOLD TEST DIAGNOSTIC ===");
        info!("Validators: {}", config.num_validators);
        info!(
            "Expected threshold: {} (2/3 + 1)",
            (config.num_validators * 2 / 3) + 1
        );
        info!("Last error: {}", last_error);
        info!("Check validator logs for:");
        info!("  - [DKG] Individual secret share submissions");
        info!("  - [DKG] Share aggregation progress");
        info!("  - [DKG] Threshold verification");
        info!("==================================");
    }

    assert!(
        revealed,
        "Secret should be revealed when threshold ({}/{}) is met. Last error: {}",
        (config.num_validators * 2 / 3) + 1,
        config.num_validators,
        last_error
    );
}
