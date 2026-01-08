//! IBE (Identity-Based Encryption) tests for timelock (Registry Model)

use super::test_helpers::{create_timelock_swarm, register_timelock, TimelockTestConfig};
use aptos_api_types::ViewFunction;
use aptos_dkg::ibe::{compute_timelock_identity, ibe_decrypt, ibe_encrypt};
use aptos_dkg::pvss::traits::Transcript;
use aptos_logger::info;
use aptos_types::dkg::real_dkg::Transcripts;
use blstrs::G1Projective;
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

/*
async fn log_dkg_state(client: &aptos_rest_client::Client) {
    info!("=== DKG State Diagnostic ===");
    match super::verify_public_key_published(client, 1).await {
        Ok(bytes) => {
            info!("DKG transcript available: {} bytes", bytes.len());
        },
        Err(e) => {
            info!("DKG transcript NOT available: {}", e);
        },
    }

    match super::verify_master_public_key_on_chain(client, 1).await {
        Ok(bytes) => {
            info!("Master Public Key available: {} bytes", bytes.len());
        },
        Err(e) => {
            info!("Master Public Key NOT available: {}", e);
        },
    }
    info!("===========================");
}
*/

#[tokio::test]
async fn test_ibe_registry_e2e() {
    let config = TimelockTestConfig::default();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("1. Getting MPK...");
    let mut transcript_bytes = Vec::new();
    let mut mpk_attempts = 0;
    for attempt in 0..60 {
        match super::verify_public_key_published(&client, 1).await {
            Ok(bytes) => {
                transcript_bytes = bytes;
                info!(
                    "✅ Successfully retrieved MPK transcript on attempt {}/60: {} bytes",
                    attempt + 1,
                    transcript_bytes.len()
                );
                break;
            },
            Err(e) => {
                mpk_attempts = attempt + 1;
                if attempt % 10 == 0 {
                    info!(
                        "Attempt {}/60: Failed to get MPK transcript: {}",
                        attempt + 1,
                        e
                    );
                }
            },
        }
        sleep(Duration::from_secs(1)).await;
    }
    assert!(
        !transcript_bytes.is_empty(),
        "Failed to get MPK transcript after {} attempts",
        mpk_attempts
    );

    let transcripts: Transcripts = bcs::from_bytes(&transcript_bytes).unwrap();
    // Use main transcript for MPK
    let mpk = transcripts.main.get_dealt_public_key();
    let mpk_g2 = mpk.as_group_element();

    // 2. Register Timelock
    let now = get_chain_time(&client).await;
    let deadline = now + 10_000_000; // 10s
    let timelock_id = 2; // Expected ID

    info!(
        "2. Registering Timelock ID {} Deadline {}",
        timelock_id, deadline
    );
    register_timelock(&*swarm, &client, deadline).await.unwrap();

    // 3. Encrypt Message
    info!("3. Encrypting...");
    let identity = compute_timelock_identity(timelock_id, deadline);
    let plaintext = b"Atomic Timelock Secret";
    let ciphertext = ibe_encrypt(&mpk_g2, &identity, plaintext).unwrap();

    // 4. Wait for deadline
    info!("4. Waiting for deadline...");
    sleep(Duration::from_secs(15)).await;

    // 5. Get Decryption Key
    info!("5. Getting Decryption Key...");
    let mut dk_bytes = None;
    let mut last_error = String::new();
    let mut checked_ids: Vec<u64> = Vec::new();

    for attempt in 0..15 {
        match super::verify_secret_aggregated(&client, timelock_id, 1).await {
            Ok(secret) => {
                dk_bytes = Some(secret);
                info!(
                    "✅ Successfully retrieved decryption key for timelock_id {} on attempt {}/15",
                    timelock_id,
                    attempt + 1
                );
                break;
            },
            Err(e) => {
                last_error = e.to_string();
                if attempt % 5 == 0 {
                    info!(
                        "Attempt {}/15: Failed to get decryption key for ID {}: {}",
                        attempt + 1,
                        timelock_id,
                        e
                    );
                }
            },
        }
        sleep(Duration::from_secs(1)).await;
    }

    if dk_bytes.is_none() {
        // Comprehensive debug check - look for secrets at all IDs
        info!(
            "ERROR: Decryption key not revealed for timelock_id {} after 15 attempts",
            timelock_id
        );
        info!("Last error: {}", last_error);

        info!("Scanning all timelock IDs for available secrets...");
        for id in 0..10 {
            match super::verify_secret_aggregated(&client, id, 1).await {
                Ok(secret) => {
                    info!(
                        "FOUND: Secret available at timelock_id {} ({} bytes)",
                        id,
                        secret.len()
                    );
                    checked_ids.push(id);
                },
                Err(e) => {
                    if id == timelock_id {
                        info!("Not found at expected ID {}: {}", id, e);
                    }
                },
            }
        }

        // Log detailed diagnostics
        info!("=== DIAGNOSTIC INFO ===");
        info!("Expected timelock_id: {}", timelock_id);
        info!("Secret found at IDs: {:?}", checked_ids);
        info!("Deadline: {}", deadline);
        info!("Current chain time: {}", now);
        info!(
            "Time elapsed since deadline: {} seconds",
            (chrono::Utc::now().timestamp_micros() as u64 - deadline) / 1_000_000
        );
    }

    assert!(
        dk_bytes.is_some(),
        "Secret not revealed for ID {}. Checked IDs: {:?}. Last error: {}",
        timelock_id,
        checked_ids,
        last_error
    );

    // 6. Decrypt
    info!("6. Decrypting...");
    let dk_g1: G1Projective = bcs::from_bytes(&dk_bytes.unwrap()).unwrap();
    let decrypted = ibe_decrypt(&dk_g1, &ciphertext).unwrap();

    assert_eq!(decrypted, plaintext, "Decryption failed matching plaintext");
    info!("✅ IBE Registry Flow Verified Successfully!");
}
