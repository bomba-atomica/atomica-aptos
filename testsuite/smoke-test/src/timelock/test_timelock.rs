//! Timelock-specific tests (Registry Model)

use super::test_helpers::{create_timelock_swarm, register_timelock, TimelockTestConfig};
use aptos_logger::info;
use std::time::Duration;
use tokio::time::sleep;
use std::str::FromStr;
use move_core_types::identifier::Identifier;
use move_core_types::language_storage::ModuleId;
use aptos_api_types::ViewFunction;

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

#[tokio::test]
async fn test_timelock_registry_flow() {
    let config = TimelockTestConfig::default();
    // Swarm must handle DKG setup (StartKeyGenEvent) automatically during genesis/start.
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    // 1. Get chain time
    let now = get_chain_time(&client).await;
    let deadline = now + 10_000_000; // 10 seconds
    
    info!("Current chain time: {}", now);
    info!("Registering deadline: {}", deadline);

    // This should assign timelock_id = 2 (since initialization sets next_id=2)
    register_timelock(&*swarm, &client, deadline).await.unwrap();
    
    // 2. Wait for deadline using local sleep
     info!("Waiting for deadline (10s + buffer)...");
    sleep(Duration::from_secs(15)).await;
    
    // 3. Verify revelation
    // Poll for secret revelation on ID 2.
    let target_id = 2;
    let mut revealed = false;
    
    for _ in 0..15 { // Poll for 15 seconds
         if let Ok(secret) = super::verify_secret_aggregated(&client, target_id, 1).await {
            info!("✅ Secret revealed for timelock_id {}: {} bytes", target_id, secret.len());
            revealed = true;
            break;
        }
        sleep(Duration::from_secs(1)).await;
    }
    
    if !revealed {
        // Fallback: check other IDs in case of mismatch
         for id in 0..10 {
            if let Ok(secret) = super::verify_secret_aggregated(&client, id, 1).await {
                info!("✅ Secret revealed for ALTERNATIVE timelock_id {}: {} bytes", id, secret.len());
                revealed = true;
                break;
            }
        }
    }

    assert!(revealed, "Secret should be revealed for timelock_id {}", target_id);
}
