// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Tests for IBE encryption/decryption using DKG-derived scalar shares.
//!
//! These tests verify the end-to-end flow:
//! 1. DKG produces scalar shares via scalar ElGamal PVSS
//! 2. Scalar shares are reconstructed using Lagrange interpolation
//! 3. Master secret is used to derive IBE decryption key
//! 4. Encrypt/decrypt roundtrip works correctly

use super::*;
use crate::smoke_test_environment::SwarmBuilder;
use aptos_dkg::{
    ibe::{compute_identity, derive_decryption_key, ibe_decrypt, ibe_encrypt},
    pvss::{
        traits::{Reconstructable, ThresholdConfig, Transcript},
        Player,
    },
};
use aptos_forge::{LocalSwarm, NodeExt, SwarmExt};
use aptos_logger::info;
use aptos_rest_client::Client;
use aptos_types::{
    dkg::{
        real_dkg::{DealtSecretKeyShares, Transcripts},
        DKGSessionState, DKGState, DKGTrait, DefaultDKG,
    },
    on_chain_config::{OnChainConfig, OnChainRandomnessConfig},
};
use move_core_types::{account_address::AccountAddress, language_storage::CORE_CODE_ADDRESS};
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::time::Instant;

const G2_COMPRESSED_LENGTH: usize = 96;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct IBEPublicParams {
    mpk: Vec<u8>,
    epoch: u64,
}

impl OnChainConfig for IBEPublicParams {
    const MODULE_IDENTIFIER: &'static str = "ibe_config";
    const TYPE_IDENTIFIER: &'static str = "IBEPublicParams";
}

async fn get_ibe_public_params(rest_client: &Client) -> Option<IBEPublicParams> {
    rest_client
        .get_account_resource_bcs::<IBEPublicParams>(
            CORE_CODE_ADDRESS,
            IBEPublicParams::struct_tag().to_canonical_string().as_str(),
        )
        .await
        .ok()
        .map(|r| r.into_inner())
}

async fn get_dkg_state(rest_client: &Client) -> DKGState {
    rest_client
        .get_account_resource_bcs::<DKGState>(
            CORE_CODE_ADDRESS,
            DKGState::struct_tag().to_canonical_string().as_str(),
        )
        .await
        .expect("DKGState should exist")
        .into_inner()
}

async fn wait_for_dkg_finish(client: &Client, time_limit_secs: u64) -> DKGSessionState {
    let timer = Instant::now();
    loop {
        let dkg_state = get_dkg_state(client).await;
        if dkg_state.in_progress.is_none() && dkg_state.last_completed.is_some() {
            return dkg_state.last_completed.unwrap();
        }
        if timer.elapsed().as_secs() >= time_limit_secs {
            panic!("DKG did not complete within {} seconds", time_limit_secs);
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

fn get_decrypt_key_map(
    swarm: &LocalSwarm,
) -> HashMap<AccountAddress, <DefaultDKG as DKGTrait>::NewValidatorDecryptKey> {
    swarm
        .validators()
        .map(|validator| {
            let dk = validator
                .config()
                .consensus
                .safety_rules
                .initial_safety_rules_config
                .identity_blob()
                .unwrap()
                .try_into_dkg_new_validator_decrypt_key()
                .unwrap();
            (validator.peer_id(), dk)
        })
        .collect()
}

fn get_scalar_shares(
    dkg_session: &DKGSessionState,
    decrypt_key_map: &HashMap<AccountAddress, <DefaultDKG as DKGTrait>::NewValidatorDecryptKey>,
) -> Vec<(u64, DealtSecretKeyShares)> {
    let pub_params = DefaultDKG::new_public_params(&dkg_session.metadata);
    let transcript: Transcripts =
        bcs::from_bytes(&dkg_session.transcript).expect("Failed to deserialize transcript");

    let target_validators = dkg_session
        .metadata
        .target_validator_consensus_infos_cloned();

    target_validators
        .iter()
        .enumerate()
        .map(|(idx, validator_info)| {
            let dk = decrypt_key_map.get(&validator_info.address).unwrap();
            let (secret_share, _pub_key_share) = DefaultDKG::decrypt_secret_share_from_transcript(
                &pub_params,
                &transcript,
                idx as u64,
                dk,
            )
            .unwrap();
            (idx as u64, secret_share)
        })
        .collect()
}

fn deserialize_mpk(mpk_bytes: &[u8]) -> blstrs::G2Affine {
    let bytes: [u8; G2_COMPRESSED_LENGTH] = mpk_bytes.try_into().expect("MPK should be 96 bytes");
    blstrs::G2Affine::from_compressed(&bytes).expect("MPK should be valid G2 point")
}

/// Helper function to extract scalar shares from DKG output and reconstruct master secret.
fn extract_and_reconstruct_scalar_secret(
    dkg_session: &DKGSessionState,
    decrypt_key_map: &HashMap<AccountAddress, <DefaultDKG as DKGTrait>::NewValidatorDecryptKey>,
) -> blstrs::Scalar {
    let scalar_shares = get_scalar_shares(dkg_session, decrypt_key_map);
    let pub_params = DefaultDKG::new_public_params(&dkg_session.metadata);

    let scalar_share_pairs: Vec<(Player, _)> = scalar_shares
        .iter()
        .flat_map(|(idx, shares)| {
            let idx = *idx;
            shares
                .scalar
                .as_ref()
                .into_iter()
                .flat_map(move |scalar_shares| {
                    scalar_shares
                        .iter()
                        .enumerate()
                        .map(move |(j, scalar_share)| {
                            (
                                Player {
                                    id: idx as usize + j,
                                },
                                scalar_share.clone(),
                            )
                        })
                })
        })
        .collect();

    let reconstructed =
        <aptos_dkg::pvss::dealt_secret_key::scalar::DealtSecretKey as Reconstructable<
            aptos_dkg::pvss::ThresholdConfigBlstrs,
        >>::reconstruct(
            pub_params.pvss_config.wconfig.get_threshold_config(),
            &scalar_share_pairs,
        );

    reconstructed.s
}

#[tokio::test]
async fn elgamal_encrypt_decrypt() {
    let epoch_duration_secs = 20;

    let (swarm, _cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
            conf.consensus_config.enable_validator_txns();
            conf.consensus_config.disable_rand_check();
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
        }))
        .build_with_cli(0)
        .await;

    let rest_client = swarm.validators().next().unwrap().rest_client();

    info!("Waiting for DKG to complete...");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Epoch 2 taking too long");

    let dkg_session = wait_for_dkg_finish(&rest_client, 60).await;
    info!(
        "DKG completed for epoch {}",
        dkg_session.metadata.dealer_epoch
    );

    info!("Retrieving MPK from chain...");
    let ibe_params = get_ibe_public_params(&rest_client)
        .await
        .expect("IBEPublicParams should exist");

    assert_eq!(
        ibe_params.mpk.len(),
        G2_COMPRESSED_LENGTH,
        "MPK should be 96 bytes"
    );

    let mpk = deserialize_mpk(&ibe_params.mpk);
    info!("MPK retrieved and deserialized successfully");

    info!("Getting scalar shares from DKG...");
    let decrypt_key_map = get_decrypt_key_map(&swarm);

    let master_scalar = extract_and_reconstruct_scalar_secret(&dkg_session, &decrypt_key_map);
    info!("Scalar secret reconstructed successfully");

    let test_timelock_id: u64 = 12345;
    let test_deadline_us: u64 = 1_000_000_000_000;
    let identity = compute_identity(test_timelock_id, test_deadline_us);

    info!("Deriving decryption key for identity...");
    let decryption_key = derive_decryption_key(&master_scalar, &identity);
    info!("Decryption key derived successfully");

    let plaintext = b"Hello, Timelock! This is a secret message for the future.";
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);

    info!("Encrypting message with IBE...");
    let ciphertext = ibe_encrypt(&mpk, &identity, plaintext, &mut rng);
    info!(
        "Encrypted {} bytes -> ciphertext with {} byte payload",
        plaintext.len(),
        ciphertext.payload_len()
    );

    info!("Decrypting message with derived key...");
    let decrypted = ibe_decrypt(&decryption_key, &ciphertext);

    info!("Verifying decrypted message matches original...");
    assert_eq!(
        decrypted, plaintext,
        "Decrypted message does not match original plaintext"
    );
    info!("Decryption successful! Message verified.");
}

#[tokio::test]
async fn elgamal_encrypt_decrypt_with_different_identities() {
    let epoch_duration_secs = 20;

    let (swarm, _cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
            conf.consensus_config.enable_validator_txns();
            conf.consensus_config.disable_rand_check();
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
        }))
        .build_with_cli(0)
        .await;

    let rest_client = swarm.validators().next().unwrap().rest_client();

    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Epoch 2 taking too long");

    let dkg_session = wait_for_dkg_finish(&rest_client, 60).await;
    let ibe_params = get_ibe_public_params(&rest_client)
        .await
        .expect("IBEPublicParams should exist");
    let mpk = deserialize_mpk(&ibe_params.mpk);
    let decrypt_key_map = get_decrypt_key_map(&swarm);
    let master_scalar = extract_and_reconstruct_scalar_secret(&dkg_session, &decrypt_key_map);

    let mut rng = rand::rngs::StdRng::seed_from_u64(12345);

    for i in 0..3 {
        let identity = compute_identity(i, 1000000000);
        let dk = derive_decryption_key(&master_scalar, &identity);

        let plaintext = format!("Message for identity {}", i);
        let ciphertext = ibe_encrypt(&mpk, &identity, plaintext.as_bytes(), &mut rng);
        let decrypted = ibe_decrypt(&dk, &ciphertext);

        assert_eq!(decrypted, plaintext.as_bytes());
    }
}
