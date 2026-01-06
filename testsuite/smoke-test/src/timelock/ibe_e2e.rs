// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! IBE Encrypt/Decrypt E2E test
//!
//! This test verifies that we can:
//! 1. Fetch the published MPK (DKG transcript) from the chain
//! 2. Extract the actual IBE public key from the transcript
//! 3. Encrypt a message using IBE
//! 4. Wait for the DK (decryption key) to be revealed
//! 5. Successfully decrypt the message

use crate::smoke_test_environment::SwarmBuilder;
use aptos_dkg::ibe;
use aptos_forge::{NodeExt, Swarm};
use aptos_logger::info;
use aptos_types::on_chain_config::OnChainRandomnessConfig;
use move_core_types::identifier::Identifier;
use move_core_types::language_storage::ModuleId;
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;

#[tokio::test]
#[ignore]
async fn test_ibe_encrypt_decrypt_e2e() {
    let interval_secs = 5;

    info!("Starting IBE E2E test with 3 validators");

    let (swarm, _cli, _faucet) = SwarmBuilder::new_local(3)
        .with_num_fullnodes(0)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            // Enable validator transactions (required for timelock)
            conf.consensus_config.enable_validator_txns();

            // Enable randomness config (required for DKG manager to start)
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
        }))
        .build_with_cli(0)
        .await;

    let client = swarm.validators().next().unwrap().rest_client();
    let chain_id = swarm.chain_id().id();

    // 1. Configure shorter interval for testing
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

    // 2. Wait for first rotation to trigger DKG
    let initial_interval = super::get_current_interval(&client).await.unwrap();
    let target_interval = initial_interval + 1;
    info!("Waiting for rotation to interval {}", target_interval);
    super::wait_for_interval_rotation(&client, target_interval, 120)
        .await
        .unwrap();

    // 3. Fetch MPK from threshold_dsa module
    info!(
        "Waiting for Master Public Key for interval {}",
        target_interval
    );
    let mut mpk_bytes = Vec::new();
    for _ in 0..60 {
        if let Ok(bytes) = super::verify_master_public_key_on_chain(&client, target_interval).await {
            mpk_bytes = bytes;
            break;
        }
        sleep(Duration::from_secs(1)).await;
    }
    assert!(!mpk_bytes.is_empty(), "Failed to get MPK");

    // 4. Extract IBE Public Key (G2 point) from bytes
    // Note: The bytes stored in threshold_dsa are already the compressed G2 point
    let mpk_g2 = ibe::deserialize_g2(&mpk_bytes).expect("Failed to deserialize MPK");
    info!("Extracted MPK G2 point successfully");

    // 5. Encrypt a message using IBE
    let message = b"top_secret_bid_1000_atoms";
    let identity = ibe::compute_timelock_identity(target_interval, chain_id);
    let ciphertext = ibe::ibe_encrypt(&mpk_g2, &identity, message).expect("Encryption failed");
    info!(
        "Message encrypted successfully for interval {}",
        target_interval
    );

    // 6. Wait for reveal (rotation to target_interval + 1)
    let reveal_interval = target_interval + 1;
    info!(
        "Waiting for rotation to interval {} to trigger reveal",
        reveal_interval
    );
    super::wait_for_interval_rotation(&client, reveal_interval, 120)
        .await
        .unwrap();

    // 7. Fetch revealed Decryption Key (G1 point)
    info!(
        "Waiting for secret to be revealed for interval {}",
        target_interval
    );
    let mut dk_bytes = Vec::new();
    for _ in 0..60 {
        if let Ok(bytes) = super::verify_secret_aggregated(&client, target_interval, 3).await {
            dk_bytes = bytes;
            break;
        }
        sleep(Duration::from_secs(1)).await;
    }
    assert!(!dk_bytes.is_empty(), "Failed to get revealed secret");

    let dk_g1 = ibe::deserialize_g1(&dk_bytes).expect("Failed to deserialize DK");
    info!("Extracted DK G1 point successfully");

    // 8. Decrypt and verify
    let decrypted = ibe::ibe_decrypt(&dk_g1, &ciphertext).expect("Decryption failed");
    assert_eq!(
        message.as_slice(),
        decrypted.as_slice(),
        "Decrypted message mismatch!"
    );

    info!("✅ IBE E2E test passed! Message successfully encrypted and decrypted.");
}
