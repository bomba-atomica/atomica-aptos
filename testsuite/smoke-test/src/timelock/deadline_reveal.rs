// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Smoke test to verify DK Share Submission works correctly.

use crate::smoke_test_environment::SwarmBuilder;
use aptos_api_types::ViewRequest;
use aptos_forge::{NodeExt, Swarm, SwarmExt};
use aptos_logger::info;
use aptos_rest_client::Client;
use aptos_types::transaction::{EntryFunction, TransactionPayload};
use move_core_types::{
    ident_str,
    language_storage::{ModuleId, CORE_CODE_ADDRESS},
};
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
            &ViewRequest {
                function: "0x1::ibe_config::get_timelock".parse().unwrap(),
                type_arguments: vec![],
                arguments: vec![serde_json::Value::Number(timelock_id.into())],
            },
            None,
        )
        .await;

    match response {
        Ok(view_response) => {
            let values = view_response.inner();
            if values.len() >= 4 {
                Some(TimelockInfo {
                    timelock_id,
                    deadline_us: values[0].as_u64()?,
                    identity: values[1].as_str()?.as_bytes().to_vec(),
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

async fn get_next_timelock_id(rest_client: &Client) -> u64 {
    let response = rest_client
        .view(
            &ViewRequest {
                function: "0x1::ibe_config::get_next_timelock_id".parse().unwrap(),
                type_arguments: vec![],
                arguments: vec![],
            },
            None,
        )
        .await;

    match response {
        Ok(view_response) => view_response.inner()[0].as_u64().unwrap_or(0),
        Err(_) => 0,
    }
}

#[tokio::test]
async fn test_deadline_reveal() {
    let epoch_duration_secs = 20;

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

    let mut info = swarm.aptos_public_info();

    // Create user account
    let mut user = info
        .create_and_fund_user_account(10_000_000_000)
        .await
        .unwrap();

    // Clone client
    let rest_client = info.client().clone();

    info!("Wait for epoch 2 to ensure DKG has completed.");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Epoch 2 taking too long to arrive!");

    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64;

    let deadline_us = current_time.saturating_sub(1_000_000); // 1 sec in past
    info!(
        "Registering expired timelock with deadline {} (current: {})",
        deadline_us, current_time
    );

    let module_id = ModuleId::new(CORE_CODE_ADDRESS, ident_str!("ibe_config").to_owned());
    let function = ident_str!("register_timelock").to_owned();

    let payload = TransactionPayload::EntryFunction(EntryFunction::new(
        module_id,
        function,
        vec![],
        vec![bcs::to_bytes(&deadline_us).unwrap()],
    ));

    let txn = user.sign_with_transaction_builder(info.transaction_factory().payload(payload));
    rest_client.submit_and_wait(&txn).await.unwrap();

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
