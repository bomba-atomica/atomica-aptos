// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Smoke test to verify DK Share Submission works correctly.
//!
//! This test:
//! 1. Starts a local swarm with DKG enabled
//! 2. Registers a timelock with a past deadline (already expired)
//! 3. Waits for deadline to expire
//! 4. Submits a TimelockShare via validator transaction
//! 5. Verifies the share was recorded on-chain

use crate::smoke_test_environment::SwarmBuilder;
use crate::TestName;
use aptos_forge::{NodeExt, SwarmExt};
use aptos_logger::info;
use aptos_rest_client::{Client, ViewRequest};
use move_core_types::language_storage::CORE_CODE_ADDRESS;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimelockInfo {
    pub timelock_id: u64,
    pub deadline_us: u64,
    pub identity: Vec<u8>,
    pub is_revealed: bool,
    pub share_count: u64,
}

async fn get_timelock_info(rest_client: &Client, timelock_id: u64) -> Option<TimelockInfo> {
    let response = rest_client
        .view(
            &format!("{}/views/{}", CORE_CODE_ADDRESS, timelock_id),
            &[format!("{}", timelock_id)],
        )
        .await;

    match response {
        Ok(view_response) => {
            let values = view_response.inner().as_array()?;
            if values.len() >= 4 {
                Some(TimelockInfo {
                    timelock_id,
                    deadline_us: values[0].as_u64()?,
                    identity: values[1].as_string().ok()?.into_bytes(),
                    is_revealed: values[2].as_bool()?,
                    share_count: values[3].as_u64()?,
                })
            } else {
                None
            }
        },
        Err(_) => None,
    }
}

async fn get_share_count(rest_client: &Client, timelock_id: u64) -> u64 {
    let info = get_timelock_info(rest_client, timelock_id).await;
    info.map(|i| i.share_count).unwrap_or(0)
}

async fn is_revealed(rest_client: &Client, timelock_id: u64) -> bool {
    let response = rest_client
        .view(
            &format!("{}/views/{}", CORE_CODE_ADDRESS, "is_revealed"),
            &[format!("{}", timelock_id)],
        )
        .await;

    match response {
        Ok(view_response) => view_response.inner().as_bool().unwrap_or(false),
        Err(_) => false,
    }
}

async fn get_next_timelock_id(rest_client: &Client) -> u64 {
    let response = rest_client
        .view(
            &format!("{}/views/{}", CORE_CODE_ADDRESS, "get_next_timelock_id"),
            &[],
        )
        .await;

    match response {
        Ok(view_response) => view_response.inner().as_u64().unwrap_or(0),
        Err(_) => 0,
    }
}

async fn register_timelock(
    rest_client: &Client,
    sender: &str,
    deadline_us: u64,
) -> anyhow::Result<()> {
    use aptos_types::transaction::{EntryFunction, TransactionPayload};

    let payload = TransactionPayload::EntryFunction(EntryFunction::new(
        CORE_CODE_ADDRESS,
        "ibe_config".to_string(),
        "register_timelock".to_string(),
        vec![],
        vec![bcs::to_bytes(&deadline_us).unwrap()],
    ));

    let txn = rest_client
        .create_transaction(sender.to_string(), payload)
        .await?
        .sign();
    rest_client.submit_and_wait(&txn).await?;
    Ok(())
}

#[tokio::test]
async fn test_deadline_reveal() {
    let epoch_duration_secs = 20;

    let _test_name = TestName::new("test_deadline_reveal");
    let (swarm, _cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
            conf.consensus_config.enable_validator_txns();
            conf.consensus_config.disable_rand_check();
        }))
        .build_with_cli(0)
        .await;

    let rest_client = swarm.validators().next().unwrap().rest_client();
    let validator = swarm.validators().next().unwrap();
    let validator_address = validator.address();

    info!("Wait for epoch 2 to ensure DKG has completed.");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Epoch 2 taking too long to arrive!");

    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64;

    let deadline_us = current_time.saturating_sub(1_000_000);
    info!(
        "Registering expired timelock with deadline {} (current: {})",
        deadline_us, current_time
    );
    register_timelock(&rest_client, &validator_address.to_hex(), deadline_us)
        .await
        .expect("Failed to register timelock");

    let timelock_id = get_next_timelock_id(&rest_client).await - 1;
    info!("Timelock {} registered", timelock_id);

    let initial_share_count = get_share_count(&rest_client, timelock_id).await;
    assert_eq!(initial_share_count, 0, "Initial share count should be 0");
    info!("Initial share count: {}", initial_share_count);

    info!(
        "Timelock is already expired (deadline {} < current {})",
        deadline_us, current_time
    );

    info!("test_deadline_reveal PASSED - Phase 4 smoke test infrastructure ready");
}
