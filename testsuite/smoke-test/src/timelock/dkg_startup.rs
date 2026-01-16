// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Minimal DKG startup test
//!
//! This test verifies that the DKG manager starts up properly with the correct configuration.

use crate::smoke_test_environment::SwarmBuilder;
use aptos_forge::{NodeExt, SwarmExt};
use aptos_logger::info;
use aptos_types::{dkg::DKGState, on_chain_config::OnChainRandomnessConfig};
use std::{sync::Arc, time::Duration};

/// Minimal test to check if DKG manager starts.
///
/// This test:
/// - Starts a 4-validator network with randomness enabled
/// - Waits for epoch 2 (epoch 1 does DKG for epoch 2)
/// - Checks if DKG state exists on-chain
#[tokio::test]
async fn test_dkg_manager_startup() {
    let epoch_duration_secs = 20;

    info!("Building swarm with 3 validators");

    let (swarm, _cli, _faucet) = SwarmBuilder::new_local(3)
        .with_num_fullnodes(0)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;

            // Enable validator transactions (required for DKG)
            conf.consensus_config.enable_validator_txns();

            // Enable randomness config (required for DKG manager to start)
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
        }))
        .build_with_cli(0)
        .await;

    let client = swarm.validators().next().unwrap().rest_client();

    info!("Swarm started, waiting for epoch 2");

    // DKG runs at the end of epoch 1 to produce keys for epoch 2
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs * 4))
        .await
        .expect("Epoch 2 taking too long to arrive!");

    info!("Reached epoch 2, checking DKG state");

    // Check if DKG state resource exists
    let dkg_state = crate::utils::get_on_chain_resource::<DKGState>(&client).await;

    info!("DKG state found!");

    // Verify DKG has completed at least once
    assert!(
        dkg_state.last_completed.is_some(),
        "DKG should have completed at least once by epoch 2"
    );

    let last_complete = dkg_state.last_complete();
    info!(
        "DKG last completed: dealer_epoch={}, target_epoch={}, transcript_len={}",
        last_complete.metadata.dealer_epoch,
        last_complete.target_epoch(),
        last_complete.transcript.len()
    );

    info!("✅ Test completed - DKG manager started successfully");
}
