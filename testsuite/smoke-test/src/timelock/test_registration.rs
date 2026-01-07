//! Timelock Registration Tests
//!
//! These tests verify the timelock registration and deadline tracking steps.

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

async fn get_deadline(client: &aptos_rest_client::Client, timelock_id: u64) -> Option<u64> {
    let view = ViewFunction {
        module: ModuleId::from_str("0x1::timelock").unwrap(),
        function: Identifier::from_str("get_deadline").unwrap(),
        ty_args: vec![],
        args: vec![bcs::to_bytes(&timelock_id).unwrap()],
    };
    let response: Vec<Option<u64>> = client.view_bcs(&view, None).await.ok()?.into_inner();
    response.first().cloned().flatten()
}

/// Test 1: Verify that registering a timelock stores the deadline
#[tokio::test]
async fn test_register_timelock_stores_deadline() {
    let config = TimelockTestConfig::default();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    // Wait for swarm to be ready
    sleep(Duration::from_secs(5)).await;

    // Get chain time and register
    let now = get_chain_time(&client).await;
    let deadline = now + 60_000_000; // 60 seconds from now

    info!("Current chain time: {}", now);
    info!("Registering deadline: {}", deadline);

    register_timelock(&*swarm, &client, deadline).await.unwrap();

    // Verify via view function - timelock_id should be 2 (1 is MPK)
    let timelock_id = 2;
    let stored_deadline = get_deadline(&client, timelock_id).await;

    assert!(
        stored_deadline.is_some(),
        "Deadline should be stored for timelock_id {}",
        timelock_id
    );
    assert_eq!(
        stored_deadline.unwrap(),
        deadline,
        "Stored deadline should match registered deadline"
    );

    info!(
        "✅ Timelock {} registered with deadline {}",
        timelock_id, deadline
    );
}

/// Test 2: Verify that multiple timelocks get sequential IDs
#[tokio::test]
async fn test_register_multiple_timelocks() {
    let config = TimelockTestConfig::default();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    sleep(Duration::from_secs(5)).await;

    let now = get_chain_time(&client).await;

    // Register 3 timelocks with different deadlines
    let deadline1 = now + 30_000_000;
    let deadline2 = now + 60_000_000;
    let deadline3 = now + 90_000_000;

    register_timelock(&*swarm, &client, deadline1)
        .await
        .unwrap();
    register_timelock(&*swarm, &client, deadline2)
        .await
        .unwrap();
    register_timelock(&*swarm, &client, deadline3)
        .await
        .unwrap();

    // Verify all three (IDs 2, 3, 4)
    assert_eq!(get_deadline(&client, 2).await, Some(deadline1));
    assert_eq!(get_deadline(&client, 3).await, Some(deadline2));
    assert_eq!(get_deadline(&client, 4).await, Some(deadline3));

    info!("✅ Multiple timelocks registered with sequential IDs");
}
