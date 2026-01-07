//! Secret Revelation Tests
//!
//! These tests verify the decryption key share submission and aggregation steps.

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

/// Test 1: Verify that after deadline, secret is eventually revealed
///
/// This is the end-to-end test for secret revelation.
/// Prerequisite: MPK must be published (DKG must complete).
#[tokio::test]
async fn test_secret_revealed_after_deadline() {
    let config = TimelockTestConfig::default();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    // Wait for MPK first
    info!("Waiting for MPK to be published...");
    let mut mpk_ready = false;
    for _ in 0..60 {
        if super::verify_master_public_key_on_chain(&client, 1)
            .await
            .is_ok()
        {
            mpk_ready = true;
            break;
        }
        sleep(Duration::from_secs(1)).await;
    }
    assert!(
        mpk_ready,
        "MPK must be published before testing secret revelation"
    );
    info!("✅ MPK is ready");

    // Register timelock with short deadline
    let now = get_chain_time(&client).await;
    let deadline = now + 5_000_000; // 5 seconds
    let timelock_id = 2;

    info!(
        "Registering timelock {} with deadline {} (now={})",
        timelock_id, deadline, now
    );
    register_timelock(&*swarm, &client, deadline).await.unwrap();

    // Wait for deadline + buffer
    info!("Waiting for deadline to pass...");
    sleep(Duration::from_secs(10)).await;

    // Force block production
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

    // Poll for secret
    info!("Polling for secret revelation...");
    let mut revealed = false;
    let mut last_error = String::new();

    for attempt in 0..30 {
        match super::verify_secret_aggregated(&client, timelock_id, 1).await {
            Ok(secret) => {
                info!(
                    "✅ Secret revealed for timelock_id {} after {}s: {} bytes",
                    timelock_id,
                    attempt,
                    secret.len()
                );
                revealed = true;
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

    assert!(
        revealed,
        "Secret should be revealed for timelock_id {}. Last error: {}",
        timelock_id, last_error
    );
}
