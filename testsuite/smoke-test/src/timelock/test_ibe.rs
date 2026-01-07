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

#[tokio::test]
async fn test_ibe_registry_e2e() {
    let config = TimelockTestConfig::default();
    let (swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("1. Getting MPK...");
    let mut transcript_bytes = Vec::new();
    for _ in 0..60 {
        if let Ok(bytes) = super::verify_public_key_published(&client, 1).await {
            transcript_bytes = bytes;
            break;
        }
        sleep(Duration::from_secs(1)).await;
    }
    assert!(!transcript_bytes.is_empty(), "Failed to get MPK transcript");

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
    for _ in 0..15 {
        if let Ok(secret) = super::verify_secret_aggregated(&client, timelock_id, 1).await {
            dk_bytes = Some(secret);
            break;
        }
        sleep(Duration::from_secs(1)).await;
    }

    if dk_bytes.is_none() {
        // Debug check
        for id in 0..10 {
            if let Ok(_s) = super::verify_secret_aggregated(&client, id, 1).await {
                info!("WARN: Secret found at ID {} instead of {}", id, timelock_id);
            }
        }
    }
    assert!(
        dk_bytes.is_some(),
        "Secret not revealed for ID {}",
        timelock_id
    );

    // 6. Decrypt
    info!("6. Decrypting...");
    let dk_g1: G1Projective = bcs::from_bytes(&dk_bytes.unwrap()).unwrap();
    let decrypted = ibe_decrypt(&dk_g1, &ciphertext).unwrap();

    assert_eq!(decrypted, plaintext, "Decryption failed matching plaintext");
    info!("✅ IBE Registry Flow Verified Successfully!");
}
