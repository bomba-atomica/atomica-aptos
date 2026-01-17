// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Smoke test to verify that the IBE Master Public Key (MPK) is stored on-chain after DKG.
//!
//! This test:
//! 1. Starts a local swarm with randomness enabled
//! 2. Waits for epoch 2 (first epoch with DKG completion)
//! 3. Queries MPK from chain via `ibe_config::get_mpk()` view function
//! 4. Verifies MPK is 96 bytes (valid G2 compressed)
//! 5. Verifies `is_ready()` returns true
//! 6. Verifies on-chain MPK matches DKG transcript
//! 7. Verifies chain liveness

use crate::smoke_test_environment::SwarmBuilder;
use crate::utils::get_on_chain_resource;
use aptos_api_types::ViewFunction;
use aptos_dkg::pvss::traits::Transcript;
use aptos_forge::{NodeExt, SwarmExt};
use aptos_logger::info;
use aptos_rest_client::Client;
use aptos_types::{
    dkg::{real_dkg::Transcripts, DKGState},
    on_chain_config::{OnChainConfig, OnChainRandomnessConfig},
};
use move_core_types::{
    ident_str, identifier::Identifier, language_storage::ModuleId,
    language_storage::CORE_CODE_ADDRESS,
};
use serde::{Deserialize, Serialize};
use std::{str::FromStr, sync::Arc, time::Duration};

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

async fn get_is_ready(rest_client: &Client) -> bool {
    let view_function = ViewFunction {
        module: ModuleId::new(CORE_CODE_ADDRESS, ident_str!("ibe_config").into()),
        function: Identifier::from_str("is_ready").unwrap(),
        ty_args: vec![],
        args: vec![],
    };

    let result = rest_client.view_bcs::<Vec<u8>>(&view_function, None).await;
    match result {
        Ok(response) => {
            let bytes = response.inner();
            bytes.first().copied().unwrap_or(0) != 0
        },
        Err(_) => false,
    }
}

async fn get_ibe_epoch(rest_client: &Client) -> u64 {
    let view_function = ViewFunction {
        module: ModuleId::new(CORE_CODE_ADDRESS, ident_str!("ibe_config").into()),
        function: Identifier::from_str("get_epoch").unwrap(),
        ty_args: vec![],
        args: vec![],
    };

    let result = rest_client.view_bcs::<Vec<u64>>(&view_function, None).await;
    match result {
        Ok(response) => response.inner().first().copied().unwrap_or(0),
        Err(_) => 0,
    }
}

#[tokio::test]
async fn mpk_on_chain() {
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

    info!("Waiting for epoch 2 (first epoch with DKG completion)...");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Epoch 2 taking too long to arrive!");

    info!("DKG completed for epoch 2, verifying MPK is on-chain...");

    let ibe_params = get_ibe_public_params(&rest_client)
        .await
        .expect("IBEPublicParams should exist on-chain after DKG");

    assert_eq!(
        ibe_params.mpk.len(),
        G2_COMPRESSED_LENGTH,
        "MPK should be {} bytes (G2 compressed), got {} bytes",
        G2_COMPRESSED_LENGTH,
        ibe_params.mpk.len()
    );
    info!("MPK has correct length: {} bytes", ibe_params.mpk.len());

    let is_ready = get_is_ready(&rest_client).await;
    assert!(
        is_ready,
        "ibe_config::is_ready() should return true after DKG"
    );
    info!("is_ready() returns true");

    let on_chain_epoch = get_ibe_epoch(&rest_client).await;
    info!("On-chain IBE epoch: {}", on_chain_epoch);

    let dkg_state = get_on_chain_resource::<DKGState>(&rest_client).await;
    let dkg_session = dkg_state
        .last_completed
        .expect("DKG should have a completed session");

    let transcript_mpk = {
        let transcript: Transcripts =
            bcs::from_bytes(&dkg_session.transcript).expect("transcript should deserialize");
        transcript.main.get_dealt_public_key().to_bytes().to_vec()
    };

    assert_eq!(
        ibe_params.mpk, transcript_mpk,
        "On-chain MPK must match MPK extracted from DKG transcript"
    );
    info!("SUCCESS: On-chain MPK matches DKG transcript MPK");

    let info1 = rest_client
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner();
    let initial_version = info1.version;

    tokio::time::sleep(Duration::from_secs(5)).await;

    let info2 = rest_client
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner();

    assert!(
        info2.version > initial_version,
        "Chain should continue making progress: {} -> {}",
        initial_version,
        info2.version
    );
    info!(
        "Chain liveness verified: version {} -> {}, epoch {}",
        initial_version, info2.version, info2.epoch
    );

    info!("mpk_on_chain test PASSED");
}
