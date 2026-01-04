//! DKG-specific tests for timelock
//!
//! These tests verify that DKG (Distributed Key Generation) works correctly
//! for timelock encryption. DKG is the underlying mechanism that generates
//! the cryptographic keys used by the timelock system.

use crate::{smoke_test_environment::SwarmBuilder, utils::get_on_chain_resource};
use aptos_forge::{NodeExt, SwarmExt};
use aptos_logger::info;
use aptos_types::{dkg::DKGState, on_chain_config::OnChainRandomnessConfig};
use std::{sync::Arc, time::Duration};

/// Test that DKG manager starts correctly with randomness config enabled.
///
/// This is the most basic test - it verifies that enabling randomness
/// config causes the DKG manager to initialize and run a DKG session.
#[tokio::test]
async fn test_dkg_manager_starts() {
    let epoch_duration_secs = 20;

    info!("Building swarm with 4 validators");

    let (swarm, _cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
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
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Epoch 2 taking too long to arrive!");

    info!("Reached epoch 2, checking DKG state");

    // Check if DKG state resource exists
    let dkg_state = get_on_chain_resource::<DKGState>(&client).await;

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

    // Verify the transcript is non-empty
    assert!(
        !last_complete.transcript.is_empty(),
        "DKG transcript should not be empty"
    );

    info!("✅ DKG manager started and completed successfully");
}

/// Test that DKG completes successfully across multiple epochs.
///
/// This verifies that DKG continues to run as epochs progress.
#[tokio::test]
async fn test_dkg_runs_multiple_epochs() {
    let epoch_duration_secs = 20;

    info!("Building swarm for multi-epoch DKG test");

    let (swarm, _cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
            conf.consensus_config.enable_validator_txns();
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
        }))
        .build_with_cli(0)
        .await;

    let client = swarm.validators().next().unwrap().rest_client();

    // Wait for epoch 2
    info!("Waiting for epoch 2");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Epoch 2 taking too long");

    let dkg_state_epoch2 = get_on_chain_resource::<DKGState>(&client).await;
    assert!(dkg_state_epoch2.last_completed.is_some());
    let epoch2_target = dkg_state_epoch2.last_complete().target_epoch();
    info!("DKG completed for epoch {}", epoch2_target);

    // Wait for epoch 3
    info!("Waiting for epoch 3");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(3, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Epoch 3 taking too long");

    let dkg_state_epoch3 = get_on_chain_resource::<DKGState>(&client).await;
    assert!(dkg_state_epoch3.last_completed.is_some());
    let epoch3_target = dkg_state_epoch3.last_complete().target_epoch();
    info!("DKG completed for epoch {}", epoch3_target);

    // Verify DKG progressed to a new epoch
    assert!(
        epoch3_target > epoch2_target,
        "DKG should have progressed to a newer epoch"
    );

    info!("✅ DKG ran successfully across multiple epochs");
}

/// Test that DKG transcript can be deserialized correctly.
///
/// This verifies that the transcript stored on-chain is valid and can be
/// deserialized into the expected structure.
#[tokio::test]
async fn test_dkg_transcript_is_valid() {
    let epoch_duration_secs = 20;

    info!("Building swarm to test DKG transcript validity");

    let (swarm, _cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
            conf.consensus_config.enable_validator_txns();
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
        }))
        .build_with_cli(0)
        .await;

    let client = swarm.validators().next().unwrap().rest_client();

    info!("Waiting for epoch 2");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Epoch 2 taking too long");

    let dkg_state = get_on_chain_resource::<DKGState>(&client).await;
    let last_complete = dkg_state.last_complete();

    info!(
        "DKG completed for epoch {}, deserializing transcript",
        last_complete.target_epoch()
    );

    // Attempt to deserialize the transcript
    let transcripts: aptos_types::dkg::real_dkg::Transcripts =
        bcs::from_bytes(&last_complete.transcript)
            .expect("Failed to deserialize DKG transcript");

    // Main transcript is always present (not Option)
    let _main_transcript = &transcripts.main;
    info!("Successfully deserialized transcript");

    // Transcript structure is valid if deserialization succeeded
    info!("✅ DKG transcript is valid and deserializable");
}
