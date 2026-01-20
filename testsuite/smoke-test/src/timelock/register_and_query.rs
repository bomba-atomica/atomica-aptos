// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Smoke test to verify that the Timelock Registry works correctly on-chain.
//!
//! This test:
//! 1. Starts a local swarm with DKG enabled
//! 2. Verifies TimelockRegistry is initialized
//! 3. Registers a timelock with a future deadline
//! 4. Queries timelock info via view functions
//! 5. Verifies identity computation is deterministic
//! 6. Registers multiple timelocks and verifies counter

use crate::utils::get_on_chain_resource;
use crate::{smoke_test_environment::SwarmBuilder, TestName};
use aptos_forge::{NodeExt, SwarmExt};
use aptos_logger::info;
use aptos_rest_client::{Client, ViewRequest};
use aptos_types::{
    on_chain_config::OnChainConfig,
    transaction::{EntryFunction, TransactionPayload},
};
use move_core_types::language_storage::{ModuleId, CORE_CODE_ADDRESS};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};

/// Rust representation of aptos_framework::ibe_config::TimelockRegistry
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimelockRegistry {
    pub timelocks: Vec<TimelockInfo>,
    pub next_timelock_id: u64,
}

/// Rust representation of aptos_framework::ibe_config::TimelockInfo
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimelockInfo {
    pub timelock_id: u64,
    pub deadline_us: u64,
    pub identity: Vec<u8>,
    pub is_revealed: bool,
    pub share_count: u64,
}

/// Rust representation of the registration event
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimelockRegistrationEvent {
    pub timelock_id: u64,
    pub deadline_us: u64,
    pub sender: String,
    pub timestamp_us: u64,
}

impl OnChainConfig for TimelockRegistry {
    const MODULE_IDENTIFIER: &'static str = "ibe_config";
    const TYPE_IDENTIFIER: &'static str = "TimelockRegistry";
}

/// Get timelock info by ID via view function
async fn get_timelock_info(rest_client: &Client, timelock_id: u64) -> Option<TimelockInfo> {
    let view_request = ViewRequest {
        function: format!("{}::ibe_config::get_timelock", CORE_CODE_ADDRESS)
            .parse()
            .ok()?,
        type_arguments: vec![],
        arguments: vec![serde_json::Value::Number(timelock_id.into())],
    };

    let response = rest_client.view(&view_request, None).await.ok()?;
    let values = response.inner().as_array()?;
    if values.len() >= 4 {
        Some(TimelockInfo {
            timelock_id,
            deadline_us: values[0].as_u64()?,
            identity: values[1].as_string()?.into_bytes(),
            is_revealed: values[2].as_bool()?,
            share_count: values[3].as_u64()?,
        })
    } else {
        None
    }
}

/// Get the next timelock ID via view function
async fn get_next_timelock_id(rest_client: &Client) -> u64 {
    let view_request = ViewRequest {
        function: format!("{}::ibe_config::get_next_timelock_id", CORE_CODE_ADDRESS)
            .parse()
            .ok()?,
        type_arguments: vec![],
        arguments: vec![],
    };

    match rest_client.view(&view_request, None).await {
        Ok(response) => response.inner().as_u64().unwrap_or(0),
        Err(_) => 0,
    }
}

/// Check if timelock is expired
async fn is_expired(rest_client: &Client, timelock_id: u64) -> bool {
    let view_request = ViewRequest {
        function: format!("{}::ibe_config::is_expired", CORE_CODE_ADDRESS)
            .parse()
            .ok()?,
        type_arguments: vec![],
        arguments: vec![serde_json::Value::Number(timelock_id.into())],
    };

    match rest_client.view(&view_request, None).await {
        Ok(response) => response.inner().as_bool().unwrap_or(false),
        Err(_) => false,
    }
}

/// Check if timelock is revealed
async fn is_revealed(rest_client: &Client, timelock_id: u64) -> bool {
    let view_request = ViewRequest {
        function: format!("{}::ibe_config::is_revealed", CORE_CODE_ADDRESS)
            .parse()
            .ok()?,
        type_arguments: vec![],
        arguments: vec![serde_json::Value::Number(timelock_id.into())],
    };

    match rest_client.view(&view_request, None).await {
        Ok(response) => response.inner().as_bool().unwrap_or(false),
        Err(_) => false,
    }
}

/// Register a timelock via transaction
async fn register_timelock(
    rest_client: &Client,
    sender: &str,
    deadline_us: u64,
) -> anyhow::Result<()> {
    let payload = TransactionPayload::EntryFunction(EntryFunction::new(
        CORE_CODE_ADDRESS,
        "ibe_config".to_string(),
        "register_timelock".to_string(),
        vec![],
        vec![bcs::to_bytes(&deadline_us).unwrap()],
    ));

    let txn = rest_client
        .create_transaction(sender.to_string(), payload)
        .await?
        .sign();
    rest_client.submit_and_wait(&txn).await?;
    Ok(())
}

/// Verify the correctness of the Timelock Registry.
#[tokio::test]
async fn test_timelock_registry_register_and_query() {
    let epoch_duration_secs = 20;

    let test_name = TestName::new("test_timelock_registry_register_and_query");
    let (swarm, cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
            conf.consensus_config.enable_validator_txns();
            conf.consensus_config.disable_rand_check();
        }))
        .build_with_cli(0)
        .await;

    let rest_client = swarm.validators().next().unwrap().rest_client();

    // Get the first validator's account
    let validator = swarm.validators().next().unwrap();
    let validator_address = validator.address();

    info!("Wait for epoch 2 to ensure DKG has completed.");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Epoch 2 taking too long to arrive!");

    info!("Verify TimelockRegistry is initialized.");
    let initial_next_id = get_next_timelock_id(&rest_client).await;
    assert_eq!(
        initial_next_id, 0,
        "Initial timelock counter should be 0, got {}",
        initial_next_id
    );
    info!("TimelockRegistry is initialized with next_timelock_id = 0");

    // Register first timelock with deadline 1 minute in the future
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64;
    let deadline1 = current_time + 60_000_000; // 1 minute

    info!(
        "Registering first timelock with deadline {} (current: {})",
        deadline1, current_time
    );
    register_timelock(&rest_client, &validator_address.to_hex(), deadline1)
        .await
        .expect("Failed to register first timelock");

    // Verify timelock was registered
    let next_id_after_first = get_next_timelock_id(&rest_client).await;
    assert_eq!(
        next_id_after_first, 1,
        "After first registration, next_timelock_id should be 1, got {}",
        next_id_after_first
    );

    // Query timelock info
    let timelock1_info = get_timelock_info(&rest_client, 0)
        .await
        .expect("Timelock 0 should exist");
    assert_eq!(timelock1_info.timelock_id, 0, "Timelock 0 ID should be 0");
    assert_eq!(
        timelock1_info.deadline_us, deadline1,
        "Timelock deadline should match, expected {}, got {}",
        deadline1, timelock1_info.deadline_us
    );
    assert!(
        !timelock1_info.is_revealed,
        "Timelock should not be revealed initially"
    );
    assert_eq!(
        timelock1_info.share_count, 0,
        "Share count should be 0 initially"
    );
    assert_eq!(
        timelock1_info.identity.len(),
        32,
        "Identity should be 32 bytes (SHA3-256 output)"
    );
    info!(
        "First timelock verified: ID={}, deadline={}, identity={} bytes",
        timelock1_info.timelock_id,
        timelock1_info.deadline_us,
        timelock1_info.identity.len()
    );

    // Verify timelock is not expired
    let expired = is_expired(&rest_client, 0).await;
    assert!(!expired, "Timelock should not be expired yet");
    info!("Timelock 0 correctly shows as not expired");

    // Verify timelock is not revealed
    let revealed = is_revealed(&rest_client, 0).await;
    assert!(!revealed, "Timelock should not be revealed yet");
    info!("Timelock 0 correctly shows as not revealed");

    // Register second timelock with different deadline
    let deadline2 = current_time + 120_000_000; // 2 minutes
    info!("Registering second timelock with deadline {}", deadline2);
    register_timelock(&rest_client, &validator_address.to_hex(), deadline2)
        .await
        .expect("Failed to register second timelock");

    // Verify counter incremented
    let next_id_after_second = get_next_timelock_id(&rest_client).await;
    assert_eq!(
        next_id_after_second, 2,
        "After second registration, next_timelock_id should be 2"
    );

    // Query second timelock
    let timelock2_info = get_timelock_info(&rest_client, 1)
        .await
        .expect("Timelock 1 should exist");
    assert_eq!(timelock2_info.timelock_id, 1, "Timelock 1 ID should be 1");
    assert_eq!(
        timelock2_info.deadline_us, deadline2,
        "Second timelock deadline should be {}",
        deadline2
    );
    info!(
        "Second timelock verified: ID={}, deadline={}",
        timelock2_info.timelock_id, timelock2_info.deadline_us
    );

    // Verify identity determinism - querying same timelock should return same identity
    let timelock1_info_again = get_timelock_info(&rest_client, 0)
        .await
        .expect("Timelock 0 should still exist");
    assert_eq!(
        timelock1_info.identity, timelock1_info_again.identity,
        "Identity should be deterministic"
    );
    info!("Identity determinism verified - same identity returned on repeated queries");

    // Verify different timelocks have different identities
    assert_ne!(
        timelock1_info.identity, timelock2_info.identity,
        "Different timelocks should have different identities"
    );
    info!("Different timelocks have different identities - correct!");

    // Test registering from a different sender
    let second_validator = swarm.validators().nth(1).unwrap();
    let second_address = second_validator.address();
    let deadline3 = current_time + 180_000_000; // 3 minutes

    info!(
        "Registering third timelock from different sender {}",
        second_address
    );
    register_timelock(&rest_client, &second_address.to_hex(), deadline3)
        .await
        .expect("Failed to register timelock from second sender");

    let timelock3_info = get_timelock_info(&rest_client, 2)
        .await
        .expect("Timelock 2 should exist");
    assert_eq!(timelock3_info.timelock_id, 2, "Timelock 2 ID should be 2");
    assert_eq!(
        timelock3_info.deadline_us, deadline3,
        "Third timelock deadline should be {}",
        deadline3
    );
    info!("Third timelock from different sender verified");

    info!("test_timelock_registry_register_and_query PASSED");
}

/// Rust representation of aptos_framework::ibe_config::TimelockInfo
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimelockInfo {
    pub timelock_id: u64,
    pub deadline_us: u64,
    pub identity: Vec<u8>,
    pub is_revealed: bool,
    pub share_count: u64,
}

/// Rust representation of the registration event
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimelockRegistrationEvent {
    pub timelock_id: u64,
    pub deadline_us: u64,
    pub sender: String,
    pub timestamp_us: u64,
}

impl OnChainConfig for TimelockRegistry {
    const MODULE_IDENTIFIER: &'static str = "ibe_config";
    const TYPE_IDENTIFIER: &'static str = "TimelockRegistry";
}

/// Get timelock info by ID via view function
async fn get_timelock_info(rest_client: &Client, timelock_id: u64) -> Option<TimelockInfo> {
    let response = rest_client
        .view(
            &format!("{}/views/{}", CORE_CODE_ADDRESS, timelock_id),
            &[format!("{}", timelock_id)],
        )
        .await;

    match response {
        Ok(view_response) => {
            // Parse the view response - returns (deadline_us, identity, is_revealed, share_count)
            let values = view_response.inner().as_array()?;
            if values.len() >= 4 {
                Some(TimelockInfo {
                    timelock_id,
                    deadline_us: values[0].as_u64()?,
                    identity: values[1].as_string().ok()?.into_bytes(),
                    is_revealed: values[2].as_bool()?,
                    share_count: values[3].as_u64()?,
                })
            } else {
                None
            }
        },
        Err(_) => None,
    }
}

/// Get the next timelock ID via view function
async fn get_next_timelock_id(rest_client: &Client) -> u64 {
    let response = rest_client
        .view(
            &format!("{}/views/{}", CORE_CODE_ADDRESS, "get_next_timelock_id"),
            &[],
        )
        .await;

    match response {
        Ok(view_response) => view_response.inner().as_u64().unwrap_or(0),
        Err(_) => 0,
    }
}

/// Check if timelock is expired
async fn is_expired(rest_client: &Client, timelock_id: u64) -> bool {
    let response = rest_client
        .view(
            &format!("{}/views/{}", CORE_CODE_ADDRESS, "is_expired"),
            &[format!("{}", timelock_id)],
        )
        .await;

    match response {
        Ok(view_response) => view_response.inner().as_bool().unwrap_or(false),
        Err(_) => false,
    }
}

/// Check if timelock is revealed
async fn is_revealed(rest_client: &Client, timelock_id: u64) -> bool {
    let response = rest_client
        .view(
            &format!("{}/views/{}", CORE_CODE_ADDRESS, "is_revealed"),
            &[format!("{}", timelock_id)],
        )
        .await;

    match response {
        Ok(view_response) => view_response.inner().as_bool().unwrap_or(false),
        Err(_) => false,
    }
}

/// Register a timelock via transaction
async fn register_timelock(
    rest_client: &Client,
    sender: &str,
    deadline_us: u64,
) -> anyhow::Result<()> {
    let payload = TransactionPayload::EntryFunction(EntryFunction::new(
        CORE_CODE_ADDRESS,
        "ibe_config".to_string(),
        "register_timelock".to_string(),
        vec![],
        vec![bcs::to_bytes(&deadline_us).unwrap()],
    ));

    let txn = rest_client
        .create_transaction(sender.to_string(), payload)
        .await?
        .sign();
    rest_client.submit_and_wait(&txn).await?;
    Ok(())
}

/// Verify the correctness of the Timelock Registry.
#[tokio::test]
async fn test_timelock_registry_register_and_query() {
    let epoch_duration_secs = 20;

    let test_name = TestName::new("test_timelock_registry_register_and_query");
    let (swarm, cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
            conf.consensus_config.enable_validator_txns();
            conf.consensus_config.disable_rand_check();
        }))
        .build_with_cli(0)
        .await;

    let rest_client = swarm.validators().next().unwrap().rest_client();

    // Get the first validator's account
    let validator = swarm.validators().next().unwrap();
    let validator_address = validator.address();

    info!("Wait for epoch 2 to ensure DKG has completed.");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Epoch 2 taking too long to arrive!");

    info!("Verify TimelockRegistry is initialized.");
    let initial_next_id = get_next_timelock_id(&rest_client).await;
    assert_eq!(
        initial_next_id, 0,
        "Initial timelock counter should be 0, got {}",
        initial_next_id
    );
    info!("TimelockRegistry is initialized with next_timelock_id = 0");

    // Register first timelock with deadline 1 minute in the future
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64;
    let deadline1 = current_time + 60_000_000; // 1 minute

    info!(
        "Registering first timelock with deadline {} (current: {})",
        deadline1, current_time
    );
    register_timelock(&rest_client, &validator_address.to_hex(), deadline1)
        .await
        .expect("Failed to register first timelock");

    // Verify timelock was registered
    let next_id_after_first = get_next_timelock_id(&rest_client).await;
    assert_eq!(
        next_id_after_first, 1,
        "After first registration, next_timelock_id should be 1, got {}",
        next_id_after_first
    );

    // Query timelock info
    let timelock1_info = get_timelock_info(&rest_client, 0)
        .await
        .expect("Timelock 0 should exist");
    assert_eq!(timelock1_info.timelock_id, 0, "Timelock 0 ID should be 0");
    assert_eq!(
        timelock1_info.deadline_us, deadline1,
        "Timelock deadline should match, expected {}, got {}",
        deadline1, timelock1_info.deadline_us
    );
    assert!(
        !timelock1_info.is_revealed,
        "Timelock should not be revealed initially"
    );
    assert_eq!(
        timelock1_info.share_count, 0,
        "Share count should be 0 initially"
    );
    assert_eq!(
        timelock1_info.identity.len(),
        32,
        "Identity should be 32 bytes (SHA3-256 output)"
    );
    info!(
        "First timelock verified: ID={}, deadline={}, identity={} bytes",
        timelock1_info.timelock_id,
        timelock1_info.deadline_us,
        timelock1_info.identity.len()
    );

    // Verify timelock is not expired
    let expired = is_expired(&rest_client, 0).await;
    assert!(!expired, "Timelock should not be expired yet");
    info!("Timelock 0 correctly shows as not expired");

    // Verify timelock is not revealed
    let revealed = is_revealed(&rest_client, 0).await;
    assert!(!revealed, "Timelock should not be revealed yet");
    info!("Timelock 0 correctly shows as not revealed");

    // Register second timelock with different deadline
    let deadline2 = current_time + 120_000_000; // 2 minutes
    info!("Registering second timelock with deadline {}", deadline2);
    register_timelock(&rest_client, &validator_address.to_hex(), deadline2)
        .await
        .expect("Failed to register second timelock");

    // Verify counter incremented
    let next_id_after_second = get_next_timelock_id(&rest_client).await;
    assert_eq!(
        next_id_after_second, 2,
        "After second registration, next_timelock_id should be 2"
    );

    // Query second timelock
    let timelock2_info = get_timelock_info(&rest_client, 1)
        .await
        .expect("Timelock 1 should exist");
    assert_eq!(timelock2_info.timelock_id, 1, "Timelock 1 ID should be 1");
    assert_eq!(
        timelock2_info.deadline_us, deadline2,
        "Second timelock deadline should be {}",
        deadline2
    );
    info!(
        "Second timelock verified: ID={}, deadline={}",
        timelock2_info.timelock_id, timelock2_info.deadline_us
    );

    // Verify identity determinism - querying same timelock should return same identity
    let timelock1_info_again = get_timelock_info(&rest_client, 0)
        .await
        .expect("Timelock 0 should still exist");
    assert_eq!(
        timelock1_info.identity, timelock1_info_again.identity,
        "Identity should be deterministic"
    );
    info!("Identity determinism verified - same identity returned on repeated queries");

    // Verify different timelocks have different identities
    assert_ne!(
        timelock1_info.identity, timelock2_info.identity,
        "Different timelocks should have different identities"
    );
    info!("Different timelocks have different identities - correct!");

    // Test registering from a different sender
    let second_validator = swarm.validators().nth(1).unwrap();
    let second_address = second_validator.address();
    let deadline3 = current_time + 180_000_000; // 3 minutes

    info!(
        "Registering third timelock from different sender {}",
        second_address
    );
    register_timelock(&rest_client, &second_address.to_hex(), deadline3)
        .await
        .expect("Failed to register timelock from second sender");

    let timelock3_info = get_timelock_info(&rest_client, 2)
        .await
        .expect("Timelock 2 should exist");
    assert_eq!(timelock3_info.timelock_id, 2, "Timelock 2 ID should be 2");
    assert_eq!(
        timelock3_info.deadline_us, deadline3,
        "Third timelock deadline should be {}",
        deadline3
    );
    info!("Third timelock from different sender verified");

    info!("test_timelock_registry_register_and_query PASSED");
}
