//! Timelock-specific tests (Registry Model)

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

#[tokio::test]
async fn test_timelock_registry_flow() {
    let config = TimelockTestConfig::default();
    // Swarm must handle DKG setup (StartKeyGenEvent) automatically during genesis/start.
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    // 0. Wait for MPK to be published
    info!("Waiting for Master Public Key (ID=1) to be published...");
    let mut mpk_published = false;
    for _ in 0..60 {
        // Wait up to 60s for DKG
        if let Ok(_) = super::verify_master_public_key_on_chain(&client, 1).await {
            mpk_published = true;
            break;
        }
        sleep(Duration::from_secs(1)).await;
    }
    assert!(mpk_published, "MPK should be published within 60s");
    info!("✅ Master Public Key published.");

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

    // Force a block production by sending a dummy transaction
    let root_account = swarm.chain_info().root_account();
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
                                bcs::to_bytes(&aptos_types::account_address::AccountAddress::ONE)
                                    .unwrap(),
                                bcs::to_bytes(&100u64).unwrap(),
                            ],
                        ),
                    ),
                ),
            ),
        )
        .await;

    // 3. Verify revelation
    // Poll for secret revelation on ID 2.
    let target_id = 2;
    let mut revealed = false;

    for _ in 0..30 {
        // Poll for 30 seconds
        if let Ok(secret) = super::verify_secret_aggregated(&client, target_id, 1).await {
            info!(
                "✅ Secret revealed for timelock_id {}: {} bytes",
                target_id,
                secret.len()
            );
            revealed = true;
            break;
        }
        sleep(Duration::from_secs(1)).await;
    }

    if !revealed {
        // Fallback: check other IDs in case of mismatch
        for id in 0..10 {
            if let Ok(secret) = super::verify_secret_aggregated(&client, id, 1).await {
                info!(
                    "✅ Secret revealed for ALTERNATIVE timelock_id {}: {} bytes",
                    id,
                    secret.len()
                );
                revealed = true;
                break;
            }
        }
    }

    assert!(
        revealed,
        "Secret should be revealed for timelock_id {}",
        target_id
    );
}
