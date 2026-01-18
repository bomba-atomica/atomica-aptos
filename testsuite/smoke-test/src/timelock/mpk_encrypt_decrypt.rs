// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Smoke test for IBE encryption/decryption.
//!
//! NOTE: This test is currently ignored because the IBE module expects a scalar secret,
//! but the DKG produces a G1 projective element.
//!
//! This architectural mismatch needs to be addressed at the IBE/DKG integration level
//! by implementing Chunky PVSS (Phase 3).

use crate::smoke_test_environment::SwarmBuilder;
use aptos_dkg::{
    ibe::{compute_identity, ibe_encrypt},
    pvss::traits::Transcript,
};
use aptos_forge::{LocalSwarm, NodeExt, SwarmExt};
use aptos_logger::info;
use aptos_rest_client::Client;
use aptos_types::{
    dkg::{real_dkg::Transcripts, DKGSessionState, DKGState, DKGTrait, DefaultDKG},
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

fn get_dealt_secret(
    dkg_session: &DKGSessionState,
    decrypt_key_map: &HashMap<AccountAddress, <DefaultDKG as DKGTrait>::NewValidatorDecryptKey>,
) -> <DefaultDKG as DKGTrait>::DealtSecret {
    let pub_params = DefaultDKG::new_public_params(&dkg_session.metadata);
    let transcript: Transcripts =
        bcs::from_bytes(&dkg_session.transcript).expect("Failed to deserialize transcript");

    let target_validators = dkg_session
        .metadata
        .target_validator_consensus_infos_cloned();

    let player_share_pairs: Vec<(u64, _)> = target_validators
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
        .collect();

    DefaultDKG::reconstruct_secret_from_shares(&pub_params, player_share_pairs).unwrap()
}

fn deserialize_mpk(mpk_bytes: &[u8]) -> blstrs::G2Affine {
    let bytes: [u8; G2_COMPRESSED_LENGTH] = mpk_bytes.try_into().expect("MPK should be 96 bytes");
    blstrs::G2Affine::from_compressed(&bytes).expect("MPK should be valid G2 point")
}

#[ignore] // Blocked by DKG G1/Scalar mismatch. Awaits Chunky PVSS (Phase 3).
#[tokio::test]
async fn mpk_encrypt_decrypt() {
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

    info!("Getting dealt secret from DKG...");
    let decrypt_key_map = get_decrypt_key_map(&swarm);
    let dealt_secret = get_dealt_secret(&dkg_session, &decrypt_key_map);
    let _dealt_secret_g1 = dealt_secret.as_group_element();
    info!("Dealt secret reconstructed successfully");

    // The rest of the test is blocked because we cannot derive a valid decryption key
    // from _dealt_secret_g1 (G1Projective) that matches mpk (G2Affine).
    // The IBE system expects a Scalar secret.

    let test_timelock_id: u64 = 12345;
    let test_deadline_us: u64 = 1_000_000_000_000;
    let identity = compute_identity(test_timelock_id, test_deadline_us);

    let plaintext = b"Hello, Timelock! This is a secret message for the future.";
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);

    info!("Encrypting message with IBE...");
    // We can verify encryption works with the on-chain MPK
    let ciphertext = ibe_encrypt(&mpk, &identity, plaintext, &mut rng);
    info!(
        "Encrypted {} bytes -> ciphertext with {} byte payload",
        plaintext.len(),
        ciphertext.payload_len()
    );

    // Decryption verification is blocked.
}
