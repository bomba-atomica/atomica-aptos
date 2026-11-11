// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Validator attestation of Ethereum headers (fork only), modeled on `aptos-jwk-consensus`.
//!
//! Design: `atomica/docs/decisions/eth-header-attestation.md`. Each validator with `eth_header_observer`
//! enabled polls its own Ethereum RPC for the head at the configured block tag and asks every validator to
//! sign that header. A validator signs only when its own RPC returns the same header for that block number
//! and the block is at or below its own head. Signatures that hold 2/3+ of the voting power form a
//! `QuorumCertifiedEthHeader`, which goes into the validator transaction pool as `ObservedEthHeader`.

use crate::{
    epoch_manager::EpochManager, network::NetworkTask, network_interface::EthHeaderNetworkClient,
    types::EthHeaderMsg,
};
use aptos_config::config::{EthHeaderObserverConfig, SafetyRulesConfig};
use aptos_event_notifications::{
    DbBackedOnChainConfig, EventNotificationListener, ReconfigNotificationListener,
};
use aptos_network::application::interface::{NetworkClient, NetworkServiceEvents};
use aptos_types::account_address::AccountAddress;
use aptos_validator_transaction_pool::VTxnPoolState;
use tokio::runtime::Runtime;

pub mod certifier;
pub mod counters;
pub mod epoch_manager;
pub mod eth_rpc;
pub mod manager;
pub mod network;
pub mod network_interface;
pub mod observer;
pub mod types;

/// The Move event the runtime subscribes to.
pub const HEADER_CERTIFIED_EVENT: &str = "0x1::ethereum_headers::HeaderCertified";

pub fn start_eth_header_consensus_runtime(
    my_addr: AccountAddress,
    observer_config: EthHeaderObserverConfig,
    safety_rules_config: &SafetyRulesConfig,
    network_client: NetworkClient<EthHeaderMsg>,
    network_service_events: NetworkServiceEvents<EthHeaderMsg>,
    reconfig_events: ReconfigNotificationListener<DbBackedOnChainConfig>,
    certified_events: EventNotificationListener,
    vtxn_pool: VTxnPoolState,
) -> Runtime {
    let runtime = aptos_runtimes::spawn_named_runtime("eth-header".into(), Some(2));
    let (self_sender, self_receiver) = aptos_channels::new(1_024, &counters::PENDING_SELF_MESSAGES);
    let epoch_manager = EpochManager::new(
        my_addr,
        observer_config,
        safety_rules_config,
        reconfig_events,
        certified_events,
        self_sender,
        EthHeaderNetworkClient::new(network_client),
        vtxn_pool,
    );
    let (network_task, network_receiver) = NetworkTask::new(network_service_events, self_receiver);
    runtime.spawn(network_task.start());
    runtime.spawn(epoch_manager.start(network_receiver));
    runtime
}
