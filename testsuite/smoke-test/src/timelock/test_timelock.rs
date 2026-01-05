//! Timelock-specific tests
//!
//! These tests verify the timelock module functionality:
//! - Interval rotation
//! - Public key publication
//! - Secret revelation
//! - Configuration updates

use super::test_helpers::{create_timelock_swarm, TimelockTestConfig};
use aptos_logger::info;
use std::time::Duration;
use tokio::time::sleep;

/// Test that timelock module is initialized at genesis.
#[tokio::test]
async fn test_timelock_initialized_at_genesis() {
    let config = TimelockTestConfig::default();
    let (_swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Verifying timelock is initialized at genesis");

    let initialized = super::is_timelock_initialized(&client).await.unwrap();
    assert!(initialized, "Timelock should be initialized at genesis");

    let initial_interval = super::get_current_interval(&client).await.unwrap();
    info!("Initial interval: {}", initial_interval);

    // Initial interval should be 0 or low
    assert!(
        initial_interval < 10,
        "Initial interval should be low at genesis"
    );

    info!("✅ Timelock initialized at genesis");
}

/// Test that timelock intervals rotate correctly.
#[tokio::test]
async fn test_timelock_interval_rotation() {
    let config = TimelockTestConfig {
        timelock_interval_secs: Some(5),
        ..Default::default()
    };
    let (_swarm, client, _chain_id) = create_timelock_swarm(config).await;

    let initial_interval = super::get_current_interval(&client).await.unwrap();
    info!("Initial interval: {}", initial_interval);

    let target_interval = initial_interval + 1;
    info!("Waiting for rotation to interval {}", target_interval);

    let timeout_secs = 30; // Short timeout with 5-second intervals
    let state = super::wait_for_interval_rotation(&client, target_interval, timeout_secs)
        .await
        .unwrap();

    assert!(
        state.current_interval >= target_interval,
        "Should have rotated to interval {}",
        target_interval
    );

    info!(
        "✅ Successfully rotated from interval {} to {}",
        initial_interval, state.current_interval
    );
}

/// Test that timelock intervals continue rotating over multiple intervals.
#[tokio::test]
async fn test_timelock_multiple_rotations() {
    let config = TimelockTestConfig {
        timelock_interval_secs: Some(5),
        ..Default::default()
    };
    let (_swarm, client, _chain_id) = create_timelock_swarm(config).await;

    let initial_interval = super::get_current_interval(&client).await.unwrap();
    info!("Initial interval: {}", initial_interval);

    // Wait for 3 rotations
    let num_rotations = 3;
    let mut current_interval = initial_interval;

    for i in 1..=num_rotations {
        let target_interval = initial_interval + i;
        info!("Waiting for rotation {} to interval {}", i, target_interval);

        let state = super::wait_for_interval_rotation(&client, target_interval, 30)
            .await
            .unwrap();

        assert!(
            state.current_interval >= target_interval,
            "Should have rotated to interval {}",
            target_interval
        );

        current_interval = state.current_interval;
        info!("Completed rotation {}: interval {}", i, current_interval);
    }

    assert!(
        current_interval >= initial_interval + num_rotations,
        "Should have completed {} rotations",
        num_rotations
    );

    info!(
        "✅ Successfully completed {} rotations from interval {} to {}",
        num_rotations, initial_interval, current_interval
    );
}

/// Test that public keys are published for new intervals.
#[tokio::test]
async fn test_timelock_public_key_publication() {
    let config = TimelockTestConfig {
        timelock_interval_secs: Some(5),
        ..Default::default()
    };
    let (_swarm, client, _chain_id) = create_timelock_swarm(config).await;

    let initial_interval = super::get_current_interval(&client).await.unwrap();
    let target_interval = initial_interval + 1;

    info!("Waiting for rotation to interval {}", target_interval);
    super::wait_for_interval_rotation(&client, target_interval, 30)
        .await
        .unwrap();

    info!(
        "Waiting for public key to be published for interval {}",
        target_interval
    );

    let mut public_key = None;
    for _ in 0..60 {
        // Wait up to 60 seconds
        match super::verify_public_key_published(&client, target_interval).await {
            Ok(pk) => {
                public_key = Some(pk);
                break;
            },
            Err(_) => {
                sleep(Duration::from_secs(1)).await;
            },
        }
    }

    assert!(public_key.is_some(), "Public key should be published");

    let pk_bytes = public_key.unwrap();
    info!(
        "✅ Public key published for interval {}: {} bytes",
        target_interval,
        pk_bytes.len()
    );

    // Verify the public key is non-trivial
    assert!(pk_bytes.len() > 100, "Public key should be substantial");
}

/// Test that secrets are revealed for past intervals.
#[tokio::test]
async fn test_timelock_secret_revelation() {
    let config = TimelockTestConfig {
        timelock_interval_secs: Some(5),
        ..Default::default()
    };
    let (_swarm, client, _chain_id) = create_timelock_swarm(config).await;

    let initial_interval = super::get_current_interval(&client).await.unwrap();
    let target_interval = initial_interval + 1;

    info!("Waiting for rotation to interval {}", target_interval);
    super::wait_for_interval_rotation(&client, target_interval, 30)
        .await
        .unwrap();

    info!("Waiting for public key for interval {}", target_interval);
    let mut public_key_published = false;
    for _ in 0..60 {
        if super::verify_public_key_published(&client, target_interval)
            .await
            .is_ok()
        {
            public_key_published = true;
            break;
        }
        sleep(Duration::from_secs(1)).await;
    }
    assert!(
        public_key_published,
        "Public key should be published before secret reveal"
    );

    // Wait for next rotation to trigger reveal
    let reveal_interval = target_interval + 1;
    info!(
        "Waiting for rotation to interval {} to trigger reveal",
        reveal_interval
    );
    super::wait_for_interval_rotation(&client, reveal_interval, 30)
        .await
        .unwrap();

    info!(
        "Waiting for secret to be revealed for interval {}",
        target_interval
    );

    let mut secret = None;
    for _ in 0..60 {
        match super::verify_secret_aggregated(&client, target_interval, 3).await {
            Ok(s) => {
                secret = Some(s);
                break;
            },
            Err(_) => {
                sleep(Duration::from_secs(1)).await;
            },
        }
    }

    assert!(secret.is_some(), "Secret should be revealed");

    let secret_bytes = secret.unwrap();
    info!(
        "✅ Secret revealed for interval {}: {} bytes",
        target_interval,
        secret_bytes.len()
    );

    // Verify the secret is non-trivial
    assert!(secret_bytes.len() > 10, "Secret should be substantial");
}

/// Test the full timelock flow: rotation -> public key -> secret.
///
/// NOTE: This test is currently ignored because secret revelation is not yet implemented.
#[tokio::test]
#[ignore]
async fn test_timelock_full_flow() {
    let config = TimelockTestConfig {
        timelock_interval_secs: Some(5),
        ..Default::default()
    };
    let (_swarm, client, _chain_id) = create_timelock_swarm(config).await;

    let initial_interval = super::get_current_interval(&client).await.unwrap();
    info!("Starting full timelock flow from interval {}", initial_interval);

    // Step 1: Wait for interval rotation
    let target_interval = initial_interval + 1;
    info!("Step 1: Waiting for rotation to interval {}", target_interval);
    super::wait_for_interval_rotation(&client, target_interval, 30)
        .await
        .unwrap();
    info!("✓ Rotated to interval {}", target_interval);

    // Step 2: Wait for public key publication
    info!("Step 2: Waiting for public key publication");
    let mut public_key = None;
    for _ in 0..60 {
        if let Ok(pk) = super::verify_public_key_published(&client, target_interval).await {
            public_key = Some(pk);
            break;
        }
        sleep(Duration::from_secs(1)).await;
    }
    assert!(public_key.is_some(), "Public key should be published");
    info!("✓ Public key published: {} bytes", public_key.as_ref().unwrap().len());

    // Step 3: Wait for next rotation to trigger reveal
    let reveal_interval = target_interval + 1;
    info!("Step 3: Waiting for rotation to interval {} to trigger reveal", reveal_interval);
    super::wait_for_interval_rotation(&client, reveal_interval, 30)
        .await
        .unwrap();
    info!("✓ Rotated to interval {}", reveal_interval);

    // Step 4: Wait for secret revelation
    info!("Step 4: Waiting for secret revelation");
    let mut secret = None;
    for _ in 0..60 {
        if let Ok(s) = super::verify_secret_aggregated(&client, target_interval, 3).await {
            secret = Some(s);
            break;
        }
        sleep(Duration::from_secs(1)).await;
    }
    assert!(secret.is_some(), "Secret should be revealed");
    info!("✓ Secret revealed: {} bytes", secret.unwrap().len());

    info!("✅ Full timelock flow completed successfully");
}

/// Test force_rotation_for_testing entry function.
///
/// This test verifies that manual rotation works correctly by bypassing
/// the time check, which is useful for testing scenarios where we don't
/// want to wait for the configured interval duration.
#[tokio::test]
async fn test_force_rotation_for_testing() {
    let config = TimelockTestConfig {
        // Use a long interval so automatic rotation won't happen
        timelock_interval_secs: Some(3600),  // 1 hour
        ..Default::default()
    };
    let (swarm, client, chain_id) = create_timelock_swarm(config).await;

    let initial_interval = super::get_current_interval(&client).await.unwrap();
    info!("Initial interval: {}", initial_interval);

    // Get root account to call force_rotation
    let root_account = swarm.chain_info().root_account();

    // Force rotation from interval 0 to interval 1
    info!("Forcing rotation to interval {}", initial_interval + 1);

    let payload = aptos_types::transaction::TransactionPayload::EntryFunction(
        aptos_types::transaction::EntryFunction::new(
            move_core_types::language_storage::ModuleId::new(
                aptos_types::account_address::AccountAddress::ONE,
                move_core_types::identifier::Identifier::new("timelock").unwrap(),
            ),
            move_core_types::identifier::Identifier::new("force_rotation_for_testing").unwrap(),
            vec![],
            vec![],
        ),
    );

    let signed_txn = root_account.sign_with_transaction_builder(
        aptos_sdk::transaction_builder::TransactionFactory::new(chain_id)
            .payload(payload)
            .max_gas_amount(2_000_000)
            .gas_unit_price(100),
    );

    info!("Submitting force_rotation transaction...");
    let response = client.submit_and_wait(&signed_txn).await.unwrap();
    assert!(response.inner().success(), "Force rotation transaction should succeed");
    info!("Force rotation transaction succeeded");

    // Verify interval incremented
    let new_interval = super::get_current_interval(&client).await.unwrap();
    info!("New interval after force rotation: {}", new_interval);
    
    assert_eq!(
        new_interval,
        initial_interval + 1,
        "Interval should have incremented by 1"
    );

    // Force another rotation
    info!("Forcing second rotation to interval {}", new_interval + 1);
    
    let payload2 = aptos_types::transaction::TransactionPayload::EntryFunction(
        aptos_types::transaction::EntryFunction::new(
            move_core_types::language_storage::ModuleId::new(
                aptos_types::account_address::AccountAddress::ONE,
                move_core_types::identifier::Identifier::new("timelock").unwrap(),
            ),
            move_core_types::identifier::Identifier::new("force_rotation_for_testing").unwrap(),
            vec![],
            vec![],
        ),
    );

    let signed_txn2 = root_account.sign_with_transaction_builder(
        aptos_sdk::transaction_builder::TransactionFactory::new(chain_id)
            .payload(payload2)
            .max_gas_amount(2_000_000)
            .gas_unit_price(100),
    );

    let response2 = client.submit_and_wait(&signed_txn2).await.unwrap();
    assert!(response2.inner().success(), "Second force rotation should succeed");

    let final_interval = super::get_current_interval(&client).await.unwrap();
    info!("Final interval: {}", final_interval);
    
    assert_eq!(
        final_interval,
        initial_interval + 2,
        "Interval should have incremented by 2 total"
    );

    info!("✅ Force rotation working correctly: {} -> {} -> {}", 
          initial_interval, new_interval, final_interval);
}
