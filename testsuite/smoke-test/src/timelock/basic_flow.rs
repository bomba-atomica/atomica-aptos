// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Basic timelock flow E2E test
//!
//! This test verifies the end-to-end flow of timelock encryption:
//! 1. Genesis initialization of timelock system
//! 2. Interval rotation triggers DKG for new keys
//! 3. Validators publish public key for encryption
//! 4. Interval rotation triggers reveal request
//! 5. Validators reveal secret shares
//! 6. On-chain aggregation produces decryption key

use crate::smoke_test_environment::SwarmBuilder;
use aptos_forge::{NodeExt, Swarm};
use aptos_logger::info;
use move_core_types::identifier::Identifier;
use move_core_types::language_storage::ModuleId;
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;

/// Test basic timelock flow with fast interval for testing.
///
/// This test:
/// - Starts a 4-validator network
/// - Verifies timelock is initialized at genesis
/// - Waits for first rotation
/// - Verifies public key is published
/// - Waits for reveal
/// - Verifies secret is aggregated
#[tokio::test]
#[ignore]
async fn test_timelock_basic_flow() {
    let interval_secs = 5;

    info!(
        "Building swarm with 3 validators and {}-second interval",
        interval_secs
    );

    let (swarm, _cli, _faucet) = SwarmBuilder::new_local(3)
        .with_num_fullnodes(0)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            // Enable validator transactions (required for timelock)
            conf.consensus_config.enable_validator_txns();
        }))
        .build_with_cli(0)
        .await;

    let client = swarm.validators().next().unwrap().rest_client();

    // Configure shorter interval for testing
    {
        info!("Setting timelock interval to {} seconds", interval_secs);
        let root_account = swarm.chain_info().root_account();

        let interval_us: u64 = interval_secs * 1_000_000;

        let payload = aptos_types::transaction::TransactionPayload::EntryFunction(
            aptos_types::transaction::EntryFunction::new(
                ModuleId::new(
                    aptos_types::account_address::AccountAddress::ONE,
                    Identifier::new("timelock_config").unwrap(),
                ),
                Identifier::new("set_interval_for_testing").unwrap(),
                vec![],
                vec![bcs::to_bytes(&interval_us).unwrap()],
            ),
        );

        let signed_txn = root_account.sign_with_transaction_builder(
            aptos_sdk::transaction_builder::TransactionFactory::new(swarm.chain_id())
                .payload(payload)
                .max_gas_amount(2_000_000)
                .gas_unit_price(100),
        );

        client.submit_and_wait(&signed_txn).await.unwrap();
        info!("Timelock interval configured successfully");
    }

    info!("Swarm started, verifying timelock is initialized at genesis");

    // Step 1 - Verify timelock initialized at genesis
    let initialized = super::is_timelock_initialized(&client).await.unwrap();
    assert!(initialized, "Timelock should be initialized at genesis");

    let initial_interval = super::get_current_interval(&client).await.unwrap();
    info!("Initial interval: {}", initial_interval);
    // Note: initial_interval may be > 0 if genesis took time

    info!("Waiting for first interval rotation");

    // Step 2 - Wait for rotation to next interval
    // Use longer timeout since we can't configure short intervals yet
    let target_interval = initial_interval + 1;
    let timeout_secs = 120; // 2 minutes - may need adjustment

    let state = super::wait_for_interval_rotation(&client, target_interval, timeout_secs)
        .await
        .unwrap();
    assert!(
        state.current_interval >= target_interval,
        "Should have rotated to interval {}",
        target_interval
    );

    info!("First rotation complete, verifying public key published");

    // Step 3 - Verify public key for the new interval is published
    info!(
        "Waiting for public key to be published for interval {}",
        target_interval
    );
    let mut pub_key_published = false;
    for _ in 0..60 {
        // Wait up to 60 seconds
        match super::verify_master_public_key_on_chain(&client, target_interval).await {
            Ok(public_key) => {
                info!(
                    "Public key published for interval {}: {} bytes",
                    target_interval,
                    public_key.len()
                );
                // MPK is a compressed G2 point (96 bytes)
                assert_eq!(public_key.len(), 96, "MPK should be 96 bytes");
                pub_key_published = true;
                break;
            },
            Err(_) => {
                sleep(Duration::from_secs(1)).await;
            },
        }
    }
    assert!(
        pub_key_published,
        "Public key failure for interval {}",
        target_interval
    );

    // Step 4 - Verify Secret Reveal
    // Wait for next rotation (Target + 1)
    let reveal_target_interval = target_interval + 1;
    info!(
        "Waiting for rotation to interval {} to trigger reveal of interval {}",
        reveal_target_interval, target_interval
    );

    super::wait_for_interval_rotation(&client, reveal_target_interval, timeout_secs)
        .await
        .unwrap();

    // Now check if secret for `target_interval` is revealed
    info!(
        "Waiting for secret to be revealed for interval {}",
        target_interval
    );
    let mut secret_revealed = false;
    for _ in 0..60 {
        match super::verify_secret_aggregated(&client, target_interval, 3).await {
            Ok(secret) => {
                info!(
                    "Secret revealed for interval {}: {} bytes",
                    target_interval,
                    secret.len()
                );
                // Secret is a serialized Group Element (G1 or G2 or scalar)
                assert!(secret.len() > 0);
                secret_revealed = true;
                break;
            },
            Err(_) => {
                sleep(Duration::from_secs(1)).await;
            },
        }
    }
    assert!(
        secret_revealed,
        "Secret reveal failure for interval {}",
        target_interval
    );

    info!("✅ Test completed - basic timelock flow verified");
}

/// Test that timelock config can be updated on testnet (not mainnet).
///
/// TODO: Implement when timelock_config module is tested
#[tokio::test]
#[ignore]
async fn test_timelock_config_override() {
    // TODO: Verify set_interval_for_testing() works on testnet
    // TODO: Verify it aborts on mainnet (chain_id == 1)
}

/// Test that timelock handles validator set changes gracefully.
///
/// TODO: Implement when DKG integration is complete
#[tokio::test]
#[ignore]
async fn test_timelock_with_validator_changes() {
    // TODO: Start with 3 validators
    // TODO: Trigger DKG for interval 1
    // TODO: Add validator during DKG
    // TODO: Verify new validator doesn't break DKG
    // TODO: Verify reveal still works with threshold
}

/// Test that timelock handles DKG failures gracefully.
///
/// TODO: Implement when DKG integration is complete
#[tokio::test]
#[ignore]
async fn test_timelock_dkg_failure_recovery() {
    // TODO: Start with 3 validators
    // TODO: Kill 1 validator during DKG (below threshold of 3)
    // TODO: Verify DKG fails (below threshold)
    // TODO: Restart validators
    // TODO: Verify next interval DKG succeeds
}
