// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    agg_trx_producer::AggTranscriptProducer,
    dkg_manager::DKGManager,
    network::{IncomingRpcRequest, NetworkReceivers, NetworkSender},
    network_interface::DKGNetworkClient,
    DKGMessage,
};
use anyhow::{anyhow, Result};
use aptos_bounded_executor::BoundedExecutor;
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::config::{ReliableBroadcastConfig, SafetyRulesConfig};
use aptos_dkg;
use aptos_event_notifications::{
    EventNotification, EventNotificationListener, ReconfigNotification,
    ReconfigNotificationListener,
};
use aptos_infallible;
use aptos_logger::{debug, error, info, warn};
use aptos_network::{application::interface::NetworkClient, protocols::network::Event};
use aptos_reliable_broadcast::ReliableBroadcast;
use aptos_safety_rules::{safety_rules_manager::storage, PersistentSafetyStorage};
use aptos_types::{
    account_address::AccountAddress,
    dkg::{
        DKGSessionMetadata, DKGStartEvent, DKGState, DefaultDKG, KeyPublishedEvent,
        RequestRevealEvent, StartKeyGenEvent,
    },
    epoch_state::EpochState,
    on_chain_config::{
        OnChainConfigPayload, OnChainConfigProvider, OnChainConsensusConfig,
        OnChainRandomnessConfig, RandomnessConfigMoveStruct, RandomnessConfigSeqNum, ValidatorSet,
    },
    validator_txn::{Topic, ValidatorTransaction},
};
use aptos_validator_transaction_pool::VTxnPoolState;
use futures::StreamExt;
use futures_channel::oneshot;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio_retry::strategy::ExponentialBackoff;

pub struct EpochManager<P: OnChainConfigProvider> {
    // Some useful metadata
    my_addr: AccountAddress,
    epoch_state: Option<Arc<EpochState>>,

    // Inbound events
    reconfig_events: ReconfigNotificationListener<P>,
    dkg_start_events: EventNotificationListener,

    // Msgs to DKG manager
    dkg_rpc_msg_tx:
        Option<aptos_channel::Sender<AccountAddress, (AccountAddress, IncomingRpcRequest)>>,
    dkg_manager_close_tx: Option<oneshot::Sender<oneshot::Sender<()>>>,
    dkg_start_event_tx: Option<aptos_channel::Sender<(), DKGStartEvent>>,
    vtxn_pool: VTxnPoolState,

    // Network utils
    self_sender: aptos_channels::Sender<Event<DKGMessage>>,
    network_sender: DKGNetworkClient<NetworkClient<DKGMessage>>,
    rb_config: ReliableBroadcastConfig,

    // Randomness overriding.
    randomness_override_seq_num: u64,

    key_storage: PersistentSafetyStorage,

    // Timelock DKG sessions
    // Track close channels for active timelock DKG sessions by interval number
    // Multiple intervals can have concurrent DKG sessions running
    // We store close_tx to allow graceful shutdown of each DKG session
    timelock_dkg_close_txs: HashMap<u64, oneshot::Sender<oneshot::Sender<()>>>,

    // RPC message channels for timelock DKG communication per interval
    // We need separate channels for each timelock interval since they run concurrently
    // Note: We don't store start_event_tx because we send the event immediately after spawn
    timelock_rpc_msg_txs:
        HashMap<u64, aptos_channel::Sender<AccountAddress, (AccountAddress, IncomingRpcRequest)>>,
}

impl<P: OnChainConfigProvider> EpochManager<P> {
    pub fn new(
        safety_rules_config: &SafetyRulesConfig,
        my_addr: AccountAddress,
        reconfig_events: ReconfigNotificationListener<P>,
        dkg_start_events: EventNotificationListener,
        self_sender: aptos_channels::Sender<Event<DKGMessage>>,
        network_sender: DKGNetworkClient<NetworkClient<DKGMessage>>,
        vtxn_pool: VTxnPoolState,
        rb_config: ReliableBroadcastConfig,
        randomness_override_seq_num: u64,
    ) -> Self {
        Self {
            my_addr,
            epoch_state: None,
            reconfig_events,
            dkg_start_events,
            dkg_rpc_msg_tx: None,
            dkg_manager_close_tx: None,
            self_sender,
            network_sender,
            vtxn_pool,
            dkg_start_event_tx: None,
            rb_config,
            randomness_override_seq_num,
            key_storage: storage(safety_rules_config),
            timelock_dkg_close_txs: HashMap::new(),
            timelock_rpc_msg_txs: HashMap::new(),
        }
    }

    fn process_rpc_request(
        &mut self,
        peer_id: AccountAddress,
        dkg_request: IncomingRpcRequest,
    ) -> Result<()> {
        if Some(dkg_request.msg.epoch()) == self.epoch_state.as_ref().map(|s| s.epoch) {
            // Forward to DKGManager if it is alive.
            if let Some(tx) = &self.dkg_rpc_msg_tx {
                let _ = tx.push(peer_id, (peer_id, dkg_request));
            }
        }
        Ok(())
    }

    fn on_dkg_start_notification(&mut self, notification: EventNotification) -> Result<()> {
        if let Some(tx) = self.dkg_start_event_tx.as_ref() {
            let EventNotification {
                subscribed_events, ..
            } = notification;
            for event in subscribed_events {
                if let Ok(dkg_start_event) = DKGStartEvent::try_from(&event) {
                    let _ = tx.push((), dkg_start_event);
                    return Ok(());
                } else if let Ok(timelock_start) = StartKeyGenEvent::try_from(&event) {
                    self.start_timelock_dkg(timelock_start);
                    return Ok(());
                } else if let Ok(timelock_key) = KeyPublishedEvent::try_from(&event) {
                    self.process_timelock_key_published(timelock_key);
                    return Ok(());
                } else if let Ok(timelock_reveal) = RequestRevealEvent::try_from(&event) {
                    self.process_timelock_reveal(timelock_reveal);
                    return Ok(());
                } else {
                    debug!("[DKG] on_dkg_start_notification: failed in converting a contract event to a dkg start event!");
                }
            }
        }
        Ok(())
    }

    pub async fn start(mut self, mut network_receivers: NetworkReceivers) {
        self.await_reconfig_notification().await;
        loop {
            let handling_result = tokio::select! {
                notification = self.dkg_start_events.select_next_some() => {
                    self.on_dkg_start_notification(notification)
                },
                reconfig_notification = self.reconfig_events.select_next_some() => {
                    self.on_new_epoch(reconfig_notification).await
                },
                (peer, rpc_request) = network_receivers.rpc_rx.select_next_some() => {
                    self.process_rpc_request(peer, rpc_request)
                },
            };

            if let Err(e) = handling_result {
                error!("{}", e);
            }
        }
    }

    async fn await_reconfig_notification(&mut self) {
        let reconfig_notification = self
            .reconfig_events
            .next()
            .await
            .expect("Reconfig sender dropped, unable to start new epoch");
        self.start_new_epoch(reconfig_notification.on_chain_configs)
            .await
            .unwrap();
    }

    async fn start_new_epoch(&mut self, payload: OnChainConfigPayload<P>) -> Result<()> {
        let validator_set: ValidatorSet = payload
            .get()
            .expect("failed to get ValidatorSet from payload");

        let epoch_state = Arc::new(EpochState::new(payload.epoch(), (&validator_set).into()));
        self.epoch_state = Some(epoch_state.clone());
        let my_index = epoch_state
            .verifier
            .address_to_validator_index()
            .get(&self.my_addr)
            .copied();

        let onchain_randomness_config_seq_num = payload
            .get::<RandomnessConfigSeqNum>()
            .unwrap_or_else(|_| RandomnessConfigSeqNum::default_if_missing());

        let randomness_config_move_struct = payload.get::<RandomnessConfigMoveStruct>();

        info!(
            epoch = epoch_state.epoch,
            local = self.randomness_override_seq_num,
            onchain = onchain_randomness_config_seq_num.seq_num,
            "Checking randomness config override."
        );
        if self.randomness_override_seq_num > onchain_randomness_config_seq_num.seq_num {
            warn!("Randomness will be force-disabled by local config!");
        }

        let onchain_randomness_config = OnChainRandomnessConfig::from_configs(
            self.randomness_override_seq_num,
            onchain_randomness_config_seq_num.seq_num,
            randomness_config_move_struct.ok(),
        );

        let onchain_consensus_config: anyhow::Result<OnChainConsensusConfig> = payload.get();
        if let Err(error) = &onchain_consensus_config {
            error!("Failed to read on-chain consensus config {}", error);
        }
        let consensus_config = onchain_consensus_config.unwrap_or_default();

        // Check both validator txn and randomness features are enabled
        let randomness_enabled =
            consensus_config.is_vtxn_enabled() && onchain_randomness_config.randomness_enabled();
        if let (true, Some(my_index)) = (randomness_enabled, my_index) {
            let DKGState {
                in_progress: in_progress_session,
                ..
            } = payload.get::<DKGState>().unwrap_or_default();

            let network_sender = self.create_network_sender();
            let rb = ReliableBroadcast::new(
                self.my_addr,
                epoch_state.verifier.get_ordered_account_addresses(),
                Arc::new(network_sender),
                ExponentialBackoff::from_millis(self.rb_config.backoff_policy_base_ms)
                    .factor(self.rb_config.backoff_policy_factor)
                    .max_delay(Duration::from_millis(
                        self.rb_config.backoff_policy_max_delay_ms,
                    )),
                aptos_time_service::TimeService::real(),
                Duration::from_millis(self.rb_config.rpc_timeout_ms),
                BoundedExecutor::new(8, tokio::runtime::Handle::current()),
            );
            let agg_trx_producer = AggTranscriptProducer::new(rb);

            let (dkg_start_event_tx, dkg_start_event_rx) =
                aptos_channel::new(QueueStyle::KLAST, 1, None);
            self.dkg_start_event_tx = Some(dkg_start_event_tx);

            let (dkg_rpc_msg_tx, dkg_rpc_msg_rx) = aptos_channel::new::<
                AccountAddress,
                (AccountAddress, IncomingRpcRequest),
            >(QueueStyle::FIFO, 100, None);
            self.dkg_rpc_msg_tx = Some(dkg_rpc_msg_tx);
            let (dkg_manager_close_tx, dkg_manager_close_rx) = oneshot::channel();
            self.dkg_manager_close_tx = Some(dkg_manager_close_tx);
            let my_pk = epoch_state
                .verifier
                .get_public_key(&self.my_addr)
                .ok_or_else(|| anyhow!("my pk not found in validator set"))?;
            let dealer_sk = self.key_storage.consensus_sk_by_pk(my_pk).map_err(|e| {
                anyhow!("dkg new epoch handling failed with consensus sk lookup err: {e}")
            })?;
            let dkg_manager = DKGManager::<DefaultDKG>::new(
                Arc::new(dealer_sk),
                my_index,
                self.my_addr,
                epoch_state,
                Arc::new(agg_trx_producer),
                self.vtxn_pool.clone(),
                false,
            );
            tokio::spawn(dkg_manager.run(
                in_progress_session,
                dkg_start_event_rx,
                dkg_rpc_msg_rx,
                dkg_manager_close_rx,
            ));
        };
        Ok(())
    }

    async fn on_new_epoch(&mut self, reconfig_notification: ReconfigNotification<P>) -> Result<()> {
        self.shutdown_current_processor().await;
        self.start_new_epoch(reconfig_notification.on_chain_configs)
            .await?;
        Ok(())
    }

    async fn shutdown_current_processor(&mut self) {
        if let Some(tx) = self.dkg_manager_close_tx.take() {
            let (ack_tx, ack_rx) = oneshot::channel();
            tx.send(ack_tx).unwrap();
            ack_rx.await.unwrap();
        }
    }

    fn create_network_sender(&self) -> NetworkSender {
        NetworkSender::new(
            self.my_addr,
            self.network_sender.clone(),
            self.self_sender.clone(),
        )
    }

    /// Build DKGSessionMetadata for a timelock interval.
    ///
    /// For timelock DKG, we construct metadata from the current epoch state
    /// and the timelock configuration from the event.
    fn build_timelock_session_metadata(
        &self,
        event: &StartKeyGenEvent,
        epoch_state: &Arc<EpochState>,
    ) -> DKGSessionMetadata {
        use aptos_types::{
            on_chain_config::{OnChainRandomnessConfig, RandomnessConfigMoveStruct},
            validator_verifier::ValidatorConsensusInfoMoveStruct,
        };

        // Convert current validator set to move struct format
        let validator_consensus_infos: Vec<ValidatorConsensusInfoMoveStruct> = epoch_state
            .verifier
            .get_ordered_account_addresses_iter()
            .map(|addr| {
                let voting_power = epoch_state.verifier.get_voting_power(&addr).unwrap_or(0);
                let public_key = epoch_state
                    .verifier
                    .get_public_key(&addr)
                    .expect("public key must exist for validator");

                // Convert public key to bytes for MoveStruct
                let pk_bytes =
                    bcs::to_bytes(&public_key).expect("public key serialization should not fail");

                ValidatorConsensusInfoMoveStruct {
                    addr,
                    pk_bytes,
                    voting_power,
                }
            })
            .collect();

        // Build randomness config from timelock config
        // For timelock, we use the threshold from the event
        // Convert absolute threshold to percentage (0-100)
        let total = event.config.total_validators;
        let threshold_percentage = if total > 0 {
            (event.config.threshold * 100) / total
        } else {
            50 // Default to 50% if total is zero (shouldn't happen)
        };

        // Create RandomnessConfig using the public API
        let randomness_config_enum = OnChainRandomnessConfig::new_v1(
            threshold_percentage, // secrecy_threshold_in_percentage
            threshold_percentage, // reconstruct_threshold_in_percentage
        );

        let randomness_config = RandomnessConfigMoveStruct::from(randomness_config_enum);

        DKGSessionMetadata {
            dealer_epoch: epoch_state.epoch,
            randomness_config,
            dealer_validator_set: validator_consensus_infos.clone(),
            target_validator_set: validator_consensus_infos,
        }
    }

    fn start_timelock_dkg(&mut self, event: StartKeyGenEvent) {
        info!(
            "[Timelock] Starting DKG for interval {} (threshold={}, validators={})",
            event.interval, event.config.threshold, event.config.total_validators
        );

        // Get current epoch state - needed for validator set and network setup
        let epoch_state = match &self.epoch_state {
            Some(state) => state.clone(),
            None => {
                error!("[Timelock] Cannot start DKG - no epoch state available");
                return;
            },
        };

        // Check if we're in the current validator set
        let my_index = match epoch_state
            .verifier
            .address_to_validator_index()
            .get(&self.my_addr)
            .copied()
        {
            Some(idx) => idx,
            None => {
                warn!(
                    "[Timelock] Not participating in DKG for interval {} - not in validator set",
                    event.interval
                );
                return;
            },
        };

        // Get our consensus secret key for dealing
        let my_pk = match epoch_state.verifier.get_public_key(&self.my_addr) {
            Some(pk) => pk,
            None => {
                error!("[Timelock] Cannot find own public key in validator set");
                return;
            },
        };

        let dealer_sk = match self.key_storage.consensus_sk_by_pk(my_pk) {
            Ok(sk) => Arc::new(sk),
            Err(e) => {
                error!("[Timelock] Failed to load consensus secret key: {}", e);
                return;
            },
        };

        // Set up network components for this DKG session
        let network_sender = self.create_network_sender();
        let rb = ReliableBroadcast::new(
            self.my_addr,
            epoch_state.verifier.get_ordered_account_addresses(),
            Arc::new(network_sender),
            ExponentialBackoff::from_millis(self.rb_config.backoff_policy_base_ms)
                .factor(self.rb_config.backoff_policy_factor)
                .max_delay(Duration::from_millis(
                    self.rb_config.backoff_policy_max_delay_ms,
                )),
            aptos_time_service::TimeService::real(),
            Duration::from_millis(self.rb_config.rpc_timeout_ms),
            BoundedExecutor::new(8, tokio::runtime::Handle::current()),
        );
        let agg_trx_producer = Arc::new(AggTranscriptProducer::new(rb));

        // Create channels for this timelock DKG session
        let (start_event_tx, start_event_rx) = aptos_channel::new(QueueStyle::KLAST, 1, None);
        let (rpc_msg_tx, rpc_msg_rx) = aptos_channel::new::<
            AccountAddress,
            (AccountAddress, IncomingRpcRequest),
        >(QueueStyle::FIFO, 100, None);
        let (close_tx, close_rx) = oneshot::channel();

        // Build DKGSessionMetadata for this timelock interval
        // Note: For timelock, we use a simplified metadata structure
        // The threshold/total come from the event.config
        let session_metadata = self.build_timelock_session_metadata(&event, &epoch_state);

        // Get current timestamp for DKG start
        let start_time_us = aptos_infallible::duration_since_epoch().as_micros() as u64;

        // Create the DKGStartEvent to trigger the DKG
        let dkg_start_event = DKGStartEvent {
            session_metadata,
            start_time_us,
        };

        // Store channels for routing future messages to this interval's DKG
        self.timelock_rpc_msg_txs.insert(event.interval, rpc_msg_tx);

        // Create DKG manager with is_timelock=true
        let dkg_manager = DKGManager::<DefaultDKG>::new(
            dealer_sk,
            my_index,
            self.my_addr,
            epoch_state,
            agg_trx_producer,
            self.vtxn_pool.clone(),
            true, // is_timelock flag - tells DKGManager to submit TimelockDKGResult
        );

        // Spawn the DKG manager task
        // Note: in_progress_session is None since this is a fresh timelock DKG start
        let interval = event.interval;
        tokio::spawn(dkg_manager.run(None, start_event_rx, rpc_msg_rx, close_rx));

        // Send the start event to trigger DKG execution
        if let Err(e) = start_event_tx.push((), dkg_start_event) {
            error!(
                "[Timelock] Failed to send start event to DKG manager for interval {}: {:?}",
                interval, e
            );
            return;
        }

        // Store close channel for later cleanup
        self.timelock_dkg_close_txs.insert(interval, close_tx);

        info!(
            "[Timelock] Spawned and triggered DKG manager for interval {} (validator index {})",
            interval, my_index
        );

        // TODO Phase 3/4: After DKG completes successfully, we need to:
        // 1. Detect when the DKG transcript is finalized on-chain
        // 2. Extract our secret share from the local DKG state
        // 3. Store it using self.store_timelock_share(interval, share_bytes)
        // Options:
        //   a) Add a callback to DKGManager for completion notification
        //   b) Poll blockchain state for TimelockDKGResult events
        //   c) Have DKGManager write shares directly to storage
        // For now, this secret share extraction is deferred
    }

    fn process_timelock_key_published(&mut self, event: KeyPublishedEvent) {
        use aptos_types::dkg::{real_dkg::maybe_dk_from_bls_sk, DKGTrait, TimelockConfig};

        info!(
            "[Timelock] Processing KeyPublishedEvent for interval {}",
            event.interval
        );

        let epoch_state = match &self.epoch_state {
            Some(s) => s.clone(),
            None => {
                error!("[Timelock] Cannot process key published - no epoch state");
                return;
            },
        };

        // Reconstruct metadata
        let total = epoch_state.verifier.len() as u64;
        // Threshold: floor(N * 2 / 3) + 1
        let threshold = (total * 2 / 3) + 1;
        let config = TimelockConfig {
            threshold,
            total_validators: total,
        };
        // Create a dummy StartKeyGenEvent to reuse the metadata builder
        let start_event = StartKeyGenEvent {
            interval: event.interval,
            config,
        };

        let metadata = self.build_timelock_session_metadata(&start_event, &epoch_state);
        let pub_params = DefaultDKG::new_public_params(&metadata);

        // Deserialize transcript
        let transcript: <DefaultDKG as DKGTrait>::Transcript =
            match bcs::from_bytes(&event.public_key) {
                Ok(t) => t,
                Err(e) => {
                    error!(
                        "[Timelock] Failed to deserialize transcript for interval {}: {}",
                        event.interval, e
                    );
                    return;
                },
            };

        let my_pk = match epoch_state.verifier.get_public_key(&self.my_addr) {
            Some(pk) => pk,
            None => {
                warn!("[Timelock] My public key not found in validator set");
                return;
            },
        };

        let dealer_sk = match self.key_storage.consensus_sk_by_pk(my_pk) {
            Ok(sk) => sk,
            Err(e) => {
                error!("[Timelock] Failed to get consensus SK: {}", e);
                return;
            },
        };

        let dk = match maybe_dk_from_bls_sk(&dealer_sk) {
            Ok(dk) => dk,
            Err(e) => {
                error!("[Timelock] Failed to convert SK to DK: {}", e);
                return;
            },
        };

        let my_index = *epoch_state
            .verifier
            .address_to_validator_index()
            .get(&self.my_addr)
            .unwrap() as u64;

        let (share, _pk_share) = match DefaultDKG::decrypt_secret_share_from_transcript(
            &pub_params,
            &transcript,
            my_index,
            &dk,
        ) {
            Ok(res) => res,
            Err(e) => {
                error!(
                    "[Timelock] Failed to decrypt share for interval {}: {}",
                    event.interval, e
                );
                return;
            },
        };

        // Serialize the secret key shares
        // share.main is Vec<DealtSecretKeyShare>, each wrapping a G1Projective point
        // For timelock, we need to store these shares so we can later derive the decryption key
        // We'll serialize the entire share structure using BCS
        let share_bytes = match bcs::to_bytes(&share) {
            Ok(bytes) => bytes,
            Err(e) => {
                error!(
                    "[Timelock] Failed to serialize share for interval {}: {}",
                    event.interval, e
                );
                return;
            },
        };

        if let Err(e) = self.store_timelock_share(event.interval, &share_bytes) {
            error!("[Timelock] Failed to store share: {}", e);
        }
    }

    fn process_timelock_reveal(&self, event: RequestRevealEvent) {
        info!("[Timelock] Revealing share for interval {}", event.interval);

        // 1. Retrieve secret share from storage
        let share_bytes = match self.retrieve_timelock_share(event.interval) {
            Ok(bytes) => bytes,
            Err(e) => {
                warn!(
                    "[Timelock] Cannot reveal share for interval {}: {}",
                    event.interval, e
                );
                return;
            },
        };

        // 2. Deserialize the secret key shares
        use aptos_types::dkg::real_dkg::DealtSecretKeyShares;
        let shares: DealtSecretKeyShares = match bcs::from_bytes(&share_bytes) {
            Ok(s) => s,
            Err(e) => {
                error!(
                    "[Timelock] Failed to deserialize secret shares for interval {}: {}",
                    event.interval, e
                );
                return;
            },
        };

        // 3. Extract the G1 point from the first share
        // For timelock, we use the main path share
        // The share is already the decryption key component (G1 point)
        if shares.main.is_empty() {
            error!(
                "[Timelock] No main shares available for interval {}",
                event.interval
            );
            return;
        }

        let dk_g1 = shares.main[0].as_group_element().clone();

        // 4. Serialize decryption key to bytes (G1 compressed = 48 bytes)
        let dk_bytes = match aptos_dkg::ibe::serialize_g1(&dk_g1) {
            Ok(bytes) => bytes,
            Err(e) => {
                error!(
                    "[Timelock] Failed to serialize decryption key for interval {}: {}",
                    event.interval, e
                );
                return;
            },
        };

        // 5. Create and submit TimelockShare transaction
        let share = aptos_types::dkg::TimelockShare {
            interval: event.interval,
            author: self.my_addr,
            share: dk_bytes,
        };

        let txn = ValidatorTransaction::TimelockShare(share);
        let _guard = self.vtxn_pool.put(Topic::TIMELOCK, Arc::new(txn), None);

        info!(
            "[Timelock] Successfully computed and submitted decryption key share for interval {}",
            event.interval
        );
    }

    /// Store timelock secret share for later reveal.
    ///
    /// Currently uses in-memory cache. TODO Phase 4: Add persistent storage
    /// to survive node restarts.
    fn store_timelock_share(&mut self, interval: u64, share: &[u8]) -> Result<()> {
        info!(
            "[Timelock] Storing secret share for interval {} ({} bytes)",
            interval,
            share.len()
        );

        // Store in key_storage (persistent)
        self.key_storage
            .set_timelock_share(interval, share.to_vec())
            .map_err(|e| anyhow!("[Timelock] Failed to store share: {}", e))?;

        info!(
            "[Timelock] Share for interval {} stored successfully in persistent storage",
            interval
        );
        Ok(())
    }

    /// Retrieve stored timelock secret share.
    ///
    /// Returns error if share not found (validator may have joined after that interval).
    fn retrieve_timelock_share(&self, interval: u64) -> Result<Vec<u8>> {
        // Lookup in persistent storage
        self.key_storage
            .get_timelock_share(interval)
            .map_err(|e| {
                anyhow!(
                    "No secret share found for interval {}: {}. Validator may not have participated in DKG for this interval.",
                    interval, e
                )
            })
    }
}
