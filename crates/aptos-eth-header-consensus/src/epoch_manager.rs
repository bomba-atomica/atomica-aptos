// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Starts and stops the per-epoch [`EthHeaderManager`], modeled on the JWK consensus epoch manager.

use crate::{
    eth_rpc::JsonRpcSource,
    manager::EthHeaderManager,
    network::{IncomingRpcRequest, NetworkReceivers, NetworkSender},
    network_interface::EthHeaderNetworkClient,
    types::EthHeaderMsg,
};
use anyhow::{anyhow, Result};
use aptos_bounded_executor::BoundedExecutor;
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::config::{EthHeaderObserverConfig, SafetyRulesConfig};
use aptos_consensus_types::common::Author;
use aptos_event_notifications::{
    EventNotification, EventNotificationListener, ReconfigNotification,
    ReconfigNotificationListener,
};
use aptos_logger::{error, info};
use aptos_network::{application::interface::NetworkClient, protocols::network::Event};
use aptos_reliable_broadcast::ReliableBroadcast;
use aptos_safety_rules::{safety_rules_manager::storage, PersistentSafetyStorage};
use aptos_types::{
    account_address::AccountAddress,
    epoch_state::EpochState,
    eth_header::{EthereumHeaderConfig, EthereumHeaders, HeaderCertified},
    on_chain_config::{
        OnChainConfigPayload, OnChainConfigProvider, OnChainConsensusConfig, ValidatorSet,
    },
};
use aptos_validator_transaction_pool::VTxnPoolState;
use futures::StreamExt;
use futures_channel::oneshot;
use move_core_types::{language_storage::TypeTag, move_resource::MoveStructType};
use std::{sync::Arc, time::Duration};
use tokio_retry::strategy::ExponentialBackoff;

/// Timeout of one Ethereum RPC call.
const ETH_RPC_TIMEOUT: Duration = Duration::from_secs(10);

pub struct EpochManager<P: OnChainConfigProvider> {
    my_addr: AccountAddress,
    observer_config: EthHeaderObserverConfig,
    epoch_state: Option<Arc<EpochState>>,
    key_storage: PersistentSafetyStorage,

    reconfig_events: ReconfigNotificationListener<P>,
    certified_events: EventNotificationListener,

    certified_tx: Option<aptos_channel::Sender<(), HeaderCertified>>,
    rpc_msg_tx: Option<aptos_channel::Sender<AccountAddress, (AccountAddress, IncomingRpcRequest)>>,
    manager_close_tx: Option<oneshot::Sender<oneshot::Sender<()>>>,

    self_sender: aptos_channels::Sender<Event<EthHeaderMsg>>,
    network_sender: EthHeaderNetworkClient<NetworkClient<EthHeaderMsg>>,
    vtxn_pool: VTxnPoolState,
}

impl<P: OnChainConfigProvider> EpochManager<P> {
    pub fn new(
        my_addr: AccountAddress,
        observer_config: EthHeaderObserverConfig,
        safety_rules_config: &SafetyRulesConfig,
        reconfig_events: ReconfigNotificationListener<P>,
        certified_events: EventNotificationListener,
        self_sender: aptos_channels::Sender<Event<EthHeaderMsg>>,
        network_sender: EthHeaderNetworkClient<NetworkClient<EthHeaderMsg>>,
        vtxn_pool: VTxnPoolState,
    ) -> Self {
        Self {
            my_addr,
            observer_config,
            epoch_state: None,
            key_storage: storage(safety_rules_config),
            reconfig_events,
            certified_events,
            certified_tx: None,
            rpc_msg_tx: None,
            manager_close_tx: None,
            self_sender,
            network_sender,
            vtxn_pool,
        }
    }

    /// Forwards a peer request of the current epoch to the manager, if it runs. Otherwise the request is
    /// dropped and the peer sees an RPC error.
    fn process_rpc_request(&mut self, peer_id: Author, request: IncomingRpcRequest) {
        if Some(request.msg.epoch()) == self.epoch_state.as_ref().map(|s| s.epoch) {
            if let Some(tx) = &self.rpc_msg_tx {
                let _ = tx.push(peer_id, (peer_id, request));
            }
        }
    }

    fn process_onchain_event(&mut self, notification: EventNotification) {
        for event in notification.subscribed_events {
            let Ok(Some(certified)) = event.try_v2_typed::<HeaderCertified>(&TypeTag::Struct(
                Box::new(HeaderCertified::struct_tag()),
            )) else {
                continue;
            };
            if let Some(tx) = &self.certified_tx {
                let _ = tx.push((), certified);
            }
        }
    }

    pub async fn start(mut self, mut network_receivers: NetworkReceivers) {
        let notification = self
            .reconfig_events
            .next()
            .await
            .expect("Reconfig sender dropped, unable to start new epoch");
        if let Err(e) = self.start_new_epoch(notification.on_chain_configs) {
            error!("Ethereum header consensus failed to start: {e}");
        }
        loop {
            tokio::select! {
                notification = self.reconfig_events.select_next_some() => {
                    if let Err(e) = self.on_new_epoch(notification).await {
                        error!("Ethereum header consensus failed to start: {e}");
                    }
                },
                event = self.certified_events.select_next_some() => self.process_onchain_event(event),
                (peer, request) = network_receivers.rpc_rx.select_next_some() => {
                    self.process_rpc_request(peer, request)
                },
            }
        }
    }

    fn start_new_epoch(&mut self, payload: OnChainConfigPayload<P>) -> Result<()> {
        let validator_set: ValidatorSet = payload
            .get()
            .expect("failed to get ValidatorSet from payload");
        let epoch_state = Arc::new(EpochState::new(payload.epoch(), (&validator_set).into()));
        self.epoch_state = Some(epoch_state.clone());
        let is_validator = epoch_state
            .verifier
            .address_to_validator_index()
            .contains_key(&self.my_addr);
        let chain_config = payload.get::<EthereumHeaderConfig>().ok();
        let vtxn_enabled = payload
            .get::<OnChainConsensusConfig>()
            .unwrap_or_default()
            .is_vtxn_enabled();
        let latest_certified = payload
            .get::<EthereumHeaders>()
            .ok()
            .and_then(|h| h.latest_block_number);

        let Some(chain_config) = chain_config else {
            info!(
                epoch = epoch_state.epoch,
                "Ethereum header attestation is off on chain."
            );
            return Ok(());
        };
        if !(self.observer_config.enabled && vtxn_enabled && is_validator) {
            info!(
                epoch = epoch_state.epoch,
                observer_enabled = self.observer_config.enabled,
                vtxn_enabled = vtxn_enabled,
                is_validator = is_validator,
                "Ethereum header manager not started."
            );
            return Ok(());
        }

        let my_pk = epoch_state
            .verifier
            .get_public_key(&self.my_addr)
            .ok_or_else(|| anyhow!("my pk not found in validator set"))?;
        let my_sk = self
            .key_storage
            .consensus_sk_by_pk(my_pk)
            .map_err(|e| anyhow!("consensus sk lookup failed: {e}"))?;
        let source = JsonRpcSource::new(
            self.observer_config.rpc_url.clone(),
            self.observer_config.block_tag.clone(),
            ETH_RPC_TIMEOUT,
        )?;
        let network_sender = NetworkSender::new(
            self.my_addr,
            self.network_sender.clone(),
            self.self_sender.clone(),
        );
        let rb = ReliableBroadcast::new(
            "eth_header_consensus",
            self.my_addr,
            epoch_state.verifier.get_ordered_account_addresses(),
            Arc::new(network_sender),
            ExponentialBackoff::from_millis(5).max_delay(Duration::from_secs(10)),
            aptos_time_service::TimeService::real(),
            Duration::from_millis(15_000),
            BoundedExecutor::new(8, tokio::runtime::Handle::current()),
        );
        let (certified_tx, certified_rx) = aptos_channel::new(QueueStyle::KLAST, 1, None);
        let (rpc_msg_tx, rpc_msg_rx) = aptos_channel::new(QueueStyle::FIFO, 100, None);
        let (close_tx, close_rx) = oneshot::channel();
        self.certified_tx = Some(certified_tx);
        self.rpc_msg_tx = Some(rpc_msg_tx);
        self.manager_close_tx = Some(close_tx);

        let manager = EthHeaderManager::new(
            self.my_addr,
            epoch_state.clone(),
            chain_config.eth_chain_id,
            Arc::new(my_sk),
            Arc::new(source),
            rb,
            self.vtxn_pool.clone(),
            latest_certified,
        );
        tokio::spawn(manager.run(
            Duration::from_millis(self.observer_config.poll_interval_ms),
            certified_rx,
            rpc_msg_rx,
            close_rx,
        ));
        info!(
            epoch = epoch_state.epoch,
            eth_chain_id = chain_config.eth_chain_id,
            "Ethereum header manager spawned."
        );
        Ok(())
    }

    async fn on_new_epoch(&mut self, notification: ReconfigNotification<P>) -> Result<()> {
        if let Some(tx) = self.manager_close_tx.take() {
            let (ack_tx, ack_rx) = oneshot::channel();
            let _ = tx.send(ack_tx);
            let _ = ack_rx.await;
        }
        self.certified_tx = None;
        self.rpc_msg_tx = None;
        self.start_new_epoch(notification.on_chain_configs)
    }
}
