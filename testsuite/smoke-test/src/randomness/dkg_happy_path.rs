// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Simple sanity tests for epoch transitions and DKG.

use crate::smoke_test_environment::SwarmBuilder;
use aptos_forge::{Swarm, SwarmExt};
use aptos_types::on_chain_config::OnChainRandomnessConfig;
use std::{sync::Arc, time::Duration};

/// Test that epochs can transition without DKG when randomness is disabled.
/// This is the baseline sanity check - if this fails, the test environment is broken.
#[tokio::test]
async fn epoch_transition_without_dkg() {
    let epoch_duration_secs = 10;

    // Start swarm with randomness DISABLED - no DKG should happen
    let swarm = SwarmBuilder::new_local(3)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
            // Disable randomness and validator txns - no DKG
            conf.consensus_config.disable_validator_txns();
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_disabled());
        }))
        .build()
        .await;

    println!("Swarm started. Waiting for epoch 2 (without DKG)...");

    // This should succeed without any DKG happening
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs * 3))
        .await
        .expect("Failed to reach epoch 2 - epoch transitions broken!");

    println!("Successfully reached epoch 2 without DKG!");

    // Continue to epoch 3 to confirm it's stable
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(3, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Failed to reach epoch 3");

    println!("Successfully reached epoch 3. Epoch transitions work without DKG!");
}
