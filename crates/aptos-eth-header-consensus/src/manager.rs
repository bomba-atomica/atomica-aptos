// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Per-epoch Ethereum header consensus, modeled on the JWK `IssuerLevelConsensusManager`.
//!
//! One certification runs at a time. A new observed head replaces it. A certified header goes into the
//! validator transaction pool and stays there until a `HeaderCertified` event shows the block on chain.

use crate::{
    certifier::{sign_if_own_view_agrees, SignatureAggregation},
    eth_rpc::{EthHeader, EthHeaderSource},
    network::IncomingRpcRequest,
    observer::EthHeaderObserver,
    types::EthHeaderMsg,
};
use anyhow::anyhow;
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_crypto::bls12381::PrivateKey;
use aptos_logger::{error, info, warn};
use aptos_reliable_broadcast::ReliableBroadcast;
use aptos_types::{
    account_address::AccountAddress,
    epoch_state::EpochState,
    eth_header::{EthHeaderObservation, HeaderCertified, QuorumCertifiedEthHeader},
    validator_txn::{Topic, ValidatorTransaction},
};
use aptos_validator_transaction_pool::{TxnGuard, VTxnPoolState};
use futures_channel::oneshot;
use futures_util::{
    future::{AbortHandle, Abortable},
    FutureExt, StreamExt,
};
use std::{sync::Arc, time::Duration};
use tokio::sync::Semaphore;
use tokio_retry::strategy::ExponentialBackoff;

/// Upper bound on concurrent checks of peer sign requests. Each check makes three Ethereum RPC calls.
const MAX_CONCURRENT_SIGN_CHECKS: usize = 16;

/// Aborts the certification task when dropped.
struct AbortGuard(AbortHandle);

impl Drop for AbortGuard {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// The certification in progress. Dropping it cancels the broadcast and removes the pooled transaction.
struct Pending {
    observation: EthHeaderObservation,
    _broadcast: AbortGuard,
    vtxn_guard: Option<TxnGuard>,
}

pub struct EthHeaderManager {
    my_addr: AccountAddress,
    epoch_state: Arc<EpochState>,
    eth_chain_id: u64,
    consensus_key: Arc<PrivateKey>,
    source: Arc<dyn EthHeaderSource>,
    reliable_broadcast: Arc<ReliableBroadcast<EthHeaderMsg, ExponentialBackoff>>,
    vtxn_pool: VTxnPoolState,
    sign_checks: Arc<Semaphore>,
    /// Highest block number known to be certified on chain.
    latest_certified: Option<u64>,
    pending: Option<Pending>,
    qc_tx: aptos_channel::Sender<(), QuorumCertifiedEthHeader>,
    qc_rx: aptos_channel::Receiver<(), QuorumCertifiedEthHeader>,
}

impl EthHeaderManager {
    pub fn new(
        my_addr: AccountAddress,
        epoch_state: Arc<EpochState>,
        eth_chain_id: u64,
        consensus_key: Arc<PrivateKey>,
        source: Arc<dyn EthHeaderSource>,
        reliable_broadcast: ReliableBroadcast<EthHeaderMsg, ExponentialBackoff>,
        vtxn_pool: VTxnPoolState,
        latest_certified: Option<u64>,
    ) -> Self {
        let (qc_tx, qc_rx) = aptos_channel::new(QueueStyle::KLAST, 1, None);
        Self {
            my_addr,
            epoch_state,
            eth_chain_id,
            consensus_key,
            source,
            reliable_broadcast: Arc::new(reliable_broadcast),
            vtxn_pool,
            sign_checks: Arc::new(Semaphore::new(MAX_CONCURRENT_SIGN_CHECKS)),
            latest_certified,
            pending: None,
            qc_tx,
            qc_rx,
        }
    }

    pub async fn run(
        mut self,
        poll_interval: Duration,
        mut certified_rx: aptos_channel::Receiver<(), HeaderCertified>,
        mut rpc_req_rx: aptos_channel::Receiver<
            AccountAddress,
            (AccountAddress, IncomingRpcRequest),
        >,
        close_rx: oneshot::Receiver<oneshot::Sender<()>>,
    ) {
        let (observation_tx, mut observation_rx) = aptos_channel::new(QueueStyle::KLAST, 1, None);
        let observer = EthHeaderObserver::spawn(
            self.source.clone(),
            self.eth_chain_id,
            poll_interval,
            observation_tx,
        );
        let mut close_rx = close_rx.into_stream();
        let ack_tx = loop {
            tokio::select! {
                header = observation_rx.select_next_some() => self.process_observation(header),
                certified = self.qc_rx.select_next_some() => self.process_certified(certified),
                event = certified_rx.select_next_some() => self.process_on_chain(event),
                (_sender, request) = rpc_req_rx.select_next_some() => self.process_peer_request(request),
                ack_tx = close_rx.select_next_some() => break ack_tx.ok(),
            }
        };
        observer.shutdown().await;
        self.pending = None;
        if let Some(tx) = ack_tx {
            let _ = tx.send(());
        }
    }

    /// Starts certifying a newly observed head, unless it is already on chain or in progress.
    fn process_observation(&mut self, header: EthHeader) {
        if self.latest_certified.is_some_and(|l| header.number <= l) {
            return;
        }
        let observation = EthHeaderObservation {
            epoch: self.epoch_state.epoch,
            eth_chain_id: self.eth_chain_id,
            block_number: header.number,
            block_hash: header.hash,
            state_root: header.state_root,
        };
        if self
            .pending
            .as_ref()
            .is_some_and(|p| p.observation == observation)
        {
            return;
        }
        info!(
            epoch = self.epoch_state.epoch,
            block_number = header.number,
            "Certifying Ethereum header."
        );
        let aggregation = Arc::new(SignatureAggregation::new(
            self.epoch_state.clone(),
            observation.clone(),
        ));
        let broadcast = self
            .reliable_broadcast
            .broadcast(observation.clone(), aggregation);
        let qc_tx = self.qc_tx.clone();
        let task = async move {
            match broadcast.await {
                Ok(certified) => {
                    let _ = qc_tx.push((), certified);
                },
                Err(e) => error!("Ethereum header broadcast failed: {e}"),
            }
        };
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        tokio::spawn(Abortable::new(task, abort_registration));
        // Replacing the old pending certification cancels it and removes its pooled transaction.
        self.pending = Some(Pending {
            observation,
            _broadcast: AbortGuard(abort_handle),
            vtxn_guard: None,
        });
    }

    /// Puts a quorum-certified header of the pending observation into the validator transaction pool.
    fn process_certified(&mut self, certified: QuorumCertifiedEthHeader) {
        let Some(pending) = self.pending.as_mut() else {
            return;
        };
        if pending.observation != certified.observation || pending.vtxn_guard.is_some() {
            return;
        }
        info!(
            epoch = self.epoch_state.epoch,
            block_number = certified.observation.block_number,
            "Ethereum header certified, adding it to the validator transaction pool."
        );
        let txn = ValidatorTransaction::ObservedEthHeader(certified);
        pending.vtxn_guard = Some(self.vtxn_pool.put(Topic::ETH_HEADER, Arc::new(txn), None));
    }

    /// A header is on chain. Drops the pending certification when the chain already covers it.
    fn process_on_chain(&mut self, event: HeaderCertified) {
        let latest = self
            .latest_certified
            .map_or(event.block_number, |l| l.max(event.block_number));
        self.latest_certified = Some(latest);
        if self
            .pending
            .as_ref()
            .is_some_and(|p| p.observation.block_number <= latest)
        {
            self.pending = None;
        }
    }

    /// Checks a peer's sign request against the own Ethereum view in a separate task, then responds.
    fn process_peer_request(&self, request: IncomingRpcRequest) {
        let IncomingRpcRequest {
            msg,
            mut response_sender,
            ..
        } = request;
        let EthHeaderMsg::SignRequest(observation) = msg else {
            response_sender.send(Err(anyhow!("unexpected rpc: {}", msg.name())));
            return;
        };
        let Ok(permit) = self.sign_checks.clone().try_acquire_owned() else {
            response_sender.send(Err(anyhow!("too many pending sign requests")));
            return;
        };
        let epoch = self.epoch_state.epoch;
        let eth_chain_id = self.eth_chain_id;
        let my_addr = self.my_addr;
        let key = self.consensus_key.clone();
        let source = self.source.clone();
        tokio::spawn(async move {
            let result = sign_if_own_view_agrees(
                &observation,
                epoch,
                eth_chain_id,
                my_addr,
                &key,
                source.as_ref(),
            )
            .await;
            if let Err(e) = &result {
                warn!(
                    block_number = observation.block_number,
                    "Refusing to sign Ethereum header: {e:#}"
                );
            }
            response_sender.send(result.map(EthHeaderMsg::SignResponse));
            drop(permit);
        });
    }
}
