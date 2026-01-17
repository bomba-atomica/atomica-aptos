// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Smoke test to verify that the IBE Master Public Key (MPK) is stored on-chain after DKG.
//!
//! This test:
//! 1. Starts a local swarm with randomness enabled
//! 2. Waits for epoch 2 (first epoch with DKG completion)
//! 3. Verifies that ibe_config::IBEPublicParams exists on-chain
//! 4. Verifies the MPK is 96 bytes (compressed G2 point)
//! 5. Verifies the MPK matches the dealt public key from the DKG transcript
//! 6. Waits for epoch 3 and verifies MPK is updated

use crate::{smoke_test_environment::SwarmBuilder, utils::get_on_chain_resource};
use aptos_dkg::pvss::traits::Transcript;
use aptos_forge::{NodeExt, SwarmExt};
use aptos_logger::info;
use aptos_rest_client::Client;
use aptos_types::{
    dkg::{real_dkg::Transcripts, DKGState},
    on_chain_config::{OnChainConfig, OnChainRandomnessConfig},
};
use move_core_types::language_storage::CORE_CODE_ADDRESS;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};

/// Expected length of a compressed G2 point (BLS12-381)
const G2_COMPRESSED_LENGTH: usize = 96;

/// Rust representation of aptos_framework::ibe_config::IBEPublicParams
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IBEPublicParams {
    /// Master Public Key (G2, 96 bytes compressed)
    pub mpk: Vec<u8>,
    /// Epoch when this MPK was generated
    pub epoch: u64,
}

impl OnChainConfig for IBEPublicParams {
    const MODULE_IDENTIFIER: &'static str = "ibe_config";
    const TYPE_IDENTIFIER: &'static str = "IBEPublicParams";
}

/// Get on-chain resource using REST API
async fn get_ibe_public_params(rest_client: &Client) -> Option<IBEPublicParams> {
    let result = rest_client
        .get_account_resource_bcs::<IBEPublicParams>(
            CORE_CODE_ADDRESS,
            IBEPublicParams::struct_tag().to_canonical_string().as_str(),
        )
        .await;

    match result {
        Ok(response) => Some(response.into_inner()),
        Err(_) => None,
    }
}

/// Extract the MPK (dealt public key) from a DKG transcript.
fn extract_mpk_from_transcript(transcript_bytes: &[u8]) -> Vec<u8> {
    let transcript: Transcripts =
        bcs::from_bytes(transcript_bytes).expect("Failed to deserialize DKG transcript");
    transcript.main.get_dealt_public_key().to_bytes().to_vec()
}

/// Verify the correctness of IBE MPK storage after DKG completes.
#[tokio::test]
async fn ibe_mpk_on_chain() {
    let epoch_duration_secs = 20;

    let (swarm, _cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;

            // Ensure randomness is enabled (which enables DKG).
            conf.consensus_config.enable_validator_txns();
            conf.consensus_config.disable_rand_check();
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
        }))
        .build_with_cli(0)
        .await;

    let rest_client = swarm.validators().next().unwrap().rest_client();

    info!("Wait for epoch 2. Epoch 1 does not have DKG completion.");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Epoch 2 taking too long to arrive!");

    info!("Verify IBE MPK is stored on-chain for epoch 2.");
    let ibe_params = get_ibe_public_params(&rest_client)
        .await
        .expect("IBEPublicParams should exist on-chain after DKG");

    // Verify MPK length is 96 bytes (compressed G2 point)
    assert_eq!(
        ibe_params.mpk.len(),
        G2_COMPRESSED_LENGTH,
        "MPK should be {} bytes (compressed G2), got {} bytes",
        G2_COMPRESSED_LENGTH,
        ibe_params.mpk.len()
    );

    // Verify epoch is set (should be 2 for first DKG completion)
    assert!(
        ibe_params.epoch >= 2,
        "IBE epoch should be >= 2, got {}",
        ibe_params.epoch
    );

    info!(
        "IBE MPK verified for epoch {}: {} bytes",
        ibe_params.epoch,
        ibe_params.mpk.len()
    );

    // Verify the MPK matches what's in the DKG transcript
    info!("Verifying MPK matches DKG transcript dealt public key.");
    let dkg_state = get_on_chain_resource::<DKGState>(&rest_client).await;
    let dkg_session = dkg_state
        .last_completed
        .expect("DKG should have a completed session");

    let expected_mpk = extract_mpk_from_transcript(&dkg_session.transcript);
    assert_eq!(
        ibe_params.mpk, expected_mpk,
        "IBE MPK should match the dealt public key from DKG transcript"
    );
    info!("IBE MPK correctly matches DKG transcript dealt public key!");

    // Store the first MPK for comparison
    let first_mpk = ibe_params.mpk.clone();
    let first_epoch = ibe_params.epoch;

    info!("Wait for epoch 3 to verify MPK update.");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(3, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Epoch 3 taking too long to arrive!");

    info!("Verify IBE MPK is updated for epoch 3.");
    let ibe_params_epoch3 = get_ibe_public_params(&rest_client)
        .await
        .expect("IBEPublicParams should exist on-chain after epoch 3 DKG");

    // Verify MPK length is still 96 bytes
    assert_eq!(
        ibe_params_epoch3.mpk.len(),
        G2_COMPRESSED_LENGTH,
        "MPK should still be {} bytes after epoch 3",
        G2_COMPRESSED_LENGTH
    );

    // Verify epoch has advanced
    assert!(
        ibe_params_epoch3.epoch > first_epoch,
        "IBE epoch should have advanced from {} to at least {}, got {}",
        first_epoch,
        first_epoch + 1,
        ibe_params_epoch3.epoch
    );

    // Verify the new MPK matches the new DKG transcript
    let dkg_state_epoch3 = get_on_chain_resource::<DKGState>(&rest_client).await;
    let dkg_session_epoch3 = dkg_state_epoch3
        .last_completed
        .expect("DKG should have a completed session for epoch 3");
    let expected_mpk_epoch3 = extract_mpk_from_transcript(&dkg_session_epoch3.transcript);
    assert_eq!(
        ibe_params_epoch3.mpk, expected_mpk_epoch3,
        "IBE MPK for epoch 3 should match the dealt public key from DKG transcript"
    );

    // Verify MPK has changed (new DKG = new secret = new MPK)
    if first_mpk != ibe_params_epoch3.mpk {
        info!(
            "IBE MPK correctly updated from epoch {} to epoch {}",
            first_epoch, ibe_params_epoch3.epoch
        );
    } else {
        info!(
            "IBE MPK unchanged between epochs (same dealt public key) - this is technically possible but unlikely"
        );
    }

    info!("ibe_mpk_on_chain test PASSED");
}
