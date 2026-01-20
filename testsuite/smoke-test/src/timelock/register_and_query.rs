// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Smoke test to verify that the Timelock Registry works correctly on-chain.

use crate::smoke_test_environment::SwarmBuilder;
use aptos_api_types::ViewRequest;
use aptos_forge::{NodeExt, Swarm, SwarmExt};
use aptos_logger::info;
use aptos_rest_client::Client;
use aptos_types::transaction::{EntryFunction, TransactionPayload};
use move_core_types::{
    ident_str,
    language_storage::{ModuleId, CORE_CODE_ADDRESS},
};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};

/// Rust representation of aptos_framework::ibe_config::TimelockInfo
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimelockInfo {
    pub timelock_id: u64,
    pub deadline_us: u64,
    pub identity: Vec<u8>,
    pub is_revealed: bool,
    pub share_count: u64,
}

/// Get timelock info by ID via view function
async fn get_timelock_info(rest_client: &Client, timelock_id: u64) -> Option<TimelockInfo> {
    let response = rest_client
        .view(
            &ViewRequest {
                function: "0x1::ibe_config::get_timelock".parse().unwrap(),
                type_arguments: vec![],
                arguments: vec![serde_json::Value::Number(timelock_id.into())],
            },
            None,
        )
        .await;

    match response {
        Ok(view_response) => {
            let values = view_response.inner();
            if values.len() >= 4 {
                Some(TimelockInfo {
                    timelock_id,
                    deadline_us: values[0].as_u64()?,
                    identity: values[1].as_str()?.as_bytes().to_vec(),
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
            &ViewRequest {
                function: "0x1::ibe_config::get_next_timelock_id".parse().unwrap(),
                type_arguments: vec![],
                arguments: vec![],
            },
            None,
        )
        .await;

    match response {
        Ok(view_response) => view_response.inner()[0].as_u64().unwrap_or(0),
        Err(_) => 0,
    }
}

/// Check if timelock is expired
async fn is_expired(rest_client: &Client, timelock_id: u64) -> bool {
    let response = rest_client
        .view(
            &ViewRequest {
                function: "0x1::ibe_config::is_expired".parse().unwrap(),
                type_arguments: vec![],
                arguments: vec![serde_json::Value::Number(timelock_id.into())],
            },
            None,
        )
        .await;

    match response {
        Ok(view_response) => view_response.inner()[0].as_bool().unwrap_or(false),
        Err(_) => false,
    }
}

/// Check if timelock is revealed
async fn is_revealed(rest_client: &Client, timelock_id: u64) -> bool {
    let response = rest_client
        .view(
            &ViewRequest {
                function: "0x1::ibe_config::is_revealed".parse().unwrap(),
                type_arguments: vec![],
                arguments: vec![serde_json::Value::Number(timelock_id.into())],
            },
            None,
        )
        .await;

    match response {
        Ok(view_response) => view_response.inner()[0].as_bool().unwrap_or(false),
        Err(_) => false,
    }
}

#[tokio::test]
async fn test_timelock_registry_register_and_query() {
    let epoch_duration_secs = 20;

    let (swarm, _cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
            conf.consensus_config.enable_validator_txns();
            conf.consensus_config.disable_rand_check();
        }))
        .build_with_cli(0)
        .await;

    let mut info = swarm.aptos_public_info();

    // Create a user account to register timelocks
    let mut user = info
        .create_and_fund_user_account(10_000_000_000)
        .await
        .unwrap();

    // Clone client after creating user to avoid borrowing conflict
    let rest_client = info.client().clone();

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

    let module_id = ModuleId::new(CORE_CODE_ADDRESS, ident_str!("ibe_config").to_owned());
    let function = ident_str!("register_timelock").to_owned();

    let payload = TransactionPayload::EntryFunction(EntryFunction::new(
        module_id.clone(),
        function.clone(),
        vec![],
        vec![bcs::to_bytes(&deadline1).unwrap()],
    ));

    let txn = user.sign_with_transaction_builder(info.transaction_factory().payload(payload));
    rest_client.submit_and_wait(&txn).await.unwrap();

    // Verify timelock was registered
    let next_id_after_first = get_next_timelock_id(&rest_client).await;
    assert_eq!(
        next_id_after_first, 1,
        "After first registration, next_timelock_id should be 1"
    );

    // Query timelock info
    let timelock1_info = get_timelock_info(&rest_client, 0)
        .await
        .expect("Timelock 0 should exist");
    assert_eq!(timelock1_info.timelock_id, 0);
    assert_eq!(timelock1_info.deadline_us, deadline1);
    assert!(!timelock1_info.is_revealed);
    assert_eq!(timelock1_info.share_count, 0);
    assert_eq!(timelock1_info.identity.len(), 32);

    // Verify timelock is not expired
    assert!(!is_expired(&rest_client, 0).await);
    assert!(!is_revealed(&rest_client, 0).await);

    // Register second timelock with different deadline
    let deadline2 = current_time + 120_000_000; // 2 minutes

    let payload2 = TransactionPayload::EntryFunction(EntryFunction::new(
        module_id.clone(),
        function.clone(),
        vec![],
        vec![bcs::to_bytes(&deadline2).unwrap()],
    ));
    let txn2 = user.sign_with_transaction_builder(info.transaction_factory().payload(payload2));
    rest_client.submit_and_wait(&txn2).await.unwrap();

    // Verify counter incremented
    let next_id_after_second = get_next_timelock_id(&rest_client).await;
    assert_eq!(next_id_after_second, 2);

    // Query second timelock
    let timelock2_info = get_timelock_info(&rest_client, 1)
        .await
        .expect("Timelock 1 should exist");
    assert_eq!(timelock2_info.deadline_us, deadline2);

    // Verify identity determinism
    let timelock1_info_again = get_timelock_info(&rest_client, 0).await.unwrap();
    assert_eq!(
        timelock1_info.identity, timelock1_info_again.identity,
        "Identity should be deterministic"
    );

    // Verify different timelocks have different identities
    assert_ne!(
        timelock1_info.identity, timelock2_info.identity,
        "Different timelocks should have different identities"
    );

    info!("test_timelock_registry_register_and_query PASSED");
}
