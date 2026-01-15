// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Batch Encryption DKG Manager
//!
//! This module handles the DKG protocol for generating batch encryption keys.
//! It runs alongside the randomness DKG and uses chunky PVSS (HKZG) for threshold
//! encryption key generation.

use crate::network::NetworkSender;
use crate::network_interface::DKGNetworkClient;
use crate::{agg_trx_producer::AggTranscriptProducer, DKGMessage};
use anyhow::{anyhow, bail, ensure, Result};
use aptos_bounded_executor::BoundedExecutor;
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::config::ReliableBroadcastConfig;
use aptos_crypto::{bls12381, Uniform};
use aptos_infallible::duration_since_epoch;
use aptos_logger::{debug, error, info, warn};
use aptos_reliable_broadcast::ReliableBroadcast;
use aptos_types::{
    account_address::AccountAddress,
    dkg::batch_encryption_dkg::{
        BatchEncryptionDKGConfig, BatchEncryptionDKGPublicParams, BatchEncryptionDKGState,
        BatchEncryptionDKGTranscript,
    },
    epoch_state::EpochState,
};
use futures_channel::oneshot;
use rand::{prelude::StdRng, thread_rng, CryptoRng, RngCore, SeedableRng};
use std::{sync::Arc, time::Duration};
use tokio_retry::strategy::ExponentialBackoff;

#[derive(Clone, Debug)]
enum InnerState {
    NotStarted,
    InProgress {
        start_time: Duration,
        my_transcript: BatchEncryptionDKGTranscript,
        abort_handle: futures_util::future::AbortHandle,
    },
    Finished {
        start_time: Duration,
        my_transcript: BatchEncryptionDKGTranscript,
        config: BatchEncryptionDKGConfig,
    },
}

impl Default for InnerState {
    fn default() -> Self {
        Self::NotStarted
    }
}

#[derive(Clone)]
pub struct BatchEncryptionDKGManager {
    my_addr: AccountAddress,
    my_index: usize,
    epoch_state: Arc<EpochState>,

    dealer_sk: Arc<bls12381::PrivateKey>,
    dealer_pk: Arc<bls12381::PublicKey>,

    agg_trx_producer: Arc<dyn BatchEncryptionAggTranscriptProducer>,
    agg_trx_tx: Option<aptos_channel::Sender<(), BatchEncryptionDKGTranscript>>,

    state: InnerState,
    stopped: bool,
}

#[async_trait::async_trait]
pub trait BatchEncryptionAggTranscriptProducer: Send + Sync {
    async fn start_produce(
        &self,
        start_time: Duration,
        my_addr: AccountAddress,
        epoch_state: Arc<EpochState>,
        config: BatchEncryptionDKGConfig,
        tx: Option<aptos_channel::Sender<(), BatchEncryptionDKGTranscript>>,
    ) -> futures_util::future::AbortHandle;
}

pub struct BatchEncryptionDKGAggTranscriptProducer {
    rb: ReliableBroadcast<AccountAddress, BatchEncryptionDKGTranscript, NetworkSender>,
}

impl BatchEncryptionDKGAggTranscriptProducer {
    pub fn new(
        my_addr: AccountAddress,
        epoch_state: Arc<EpochState>,
        network_sender: NetworkSender,
        rb_config: ReliableBroadcastConfig,
    ) -> Self {
        let rb = ReliableBroadcast::new(
            my_addr,
            epoch_state.verifier.get_ordered_account_addresses(),
            Arc::new(network_sender),
            ExponentialBackoff::from_millis(rb_config.backoff_policy_base_ms)
                .factor(rb_config.backoff_policy_factor)
                .max_delay(Duration::from_millis(rb_config.backoff_policy_max_delay_ms)),
            Duration::from_millis(rb_config.rpc_timeout_ms),
            BoundedExecutor::new(8, tokio::runtime::Handle::current()),
        );
        Self { rb }
    }
}

#[async_trait::async_trait]
impl BatchEncryptionAggTranscriptProducer for BatchEncryptionDKGAggTranscriptProducer {
    async fn start_produce(
        &self,
        start_time: Duration,
        my_addr: AccountAddress,
        epoch_state: Arc<EpochState>,
        _config: BatchEncryptionDKGConfig,
        _tx: Option<aptos_channel::Sender<(), BatchEncryptionDKGTranscript>>,
    ) -> futures_util::future::AbortHandle {
        let (abort_handle, abort_registration) = futures_util::future::AbortHandle::new_pair();

        let _guard = futures_util::future::Abortable::new(
            async move {
                info!(
                    epoch = epoch_state.epoch,
                    my_addr = my_addr.to_hex(),
                    "[BatchEncryptionDKG] Starting transcript production"
                );
            },
            abort_registration,
        );

        abort_handle
    }
}

impl BatchEncryptionDKGManager {
    pub fn new(
        my_addr: AccountAddress,
        my_index: usize,
        epoch_state: Arc<EpochState>,
        dealer_sk: Arc<bls12381::PrivateKey>,
        dealer_pk: Arc<bls12381::PublicKey>,
        agg_trx_producer: Arc<dyn BatchEncryptionAggTranscriptProducer>,
    ) -> Self {
        Self {
            my_addr,
            my_index,
            epoch_state,
            dealer_sk,
            dealer_pk,
            agg_trx_producer,
            agg_trx_tx: None,
            state: InnerState::NotStarted,
            stopped: false,
        }
    }

    pub async fn run(
        mut self,
        in_progress_state: Option<(BatchEncryptionDKGState, Duration)>,
        _start_time_us: Option<u64>,
        mut rpc_msg_rx: tokio::sync::mpsc::Receiver<(
            AccountAddress,
            crate::network::IncomingRpcRequest,
        )>,
        close_rx: oneshot::Receiver<oneshot::Sender<()>>,
    ) {
        info!(
            epoch = self.epoch_state.epoch,
            my_addr = self.my_addr.to_hex().as_str(),
            "[BatchEncryptionDKG] Manager started."
        );

        let mut interval = tokio::time::interval(Duration::from_millis(5000));

        if let Some((state, _)) = in_progress_state {
            if let Some(transcript) = state.last_completed_transcript() {
                if transcript.epoch == self.epoch_state.epoch {
                    info!(
                        epoch = self.epoch_state.epoch,
                        "Found existing DKG transcript, resuming"
                    );
                }
            }
        }

        let mut close_rx = close_rx.into_stream();
        while !self.stopped {
            let handling_result = tokio::select! {
                msg = rpc_msg_rx.recv() => {
                    if let Some((peer, req)) = msg {
                        self.process_peer_rpc_msg((peer, req)).await
                            .map_err(|e| anyhow!("[BatchEncryptionDKG] process_peer_rpc_msg failed: {e}"))
                    } else {
                        Ok(())
                    }
                },
                close_req = close_rx.select_next_some() => {
                    self.process_close_cmd(close_req.ok())
                },
                _ = interval.tick().fuse() => {
                    self.observe()
                },
            };

            if let Err(e) = handling_result {
                error!(
                    epoch = self.epoch_state.epoch,
                    my_addr = self.my_addr.to_hex().as_str(),
                    "[BatchEncryptionDKG] Manager handling error: {e}"
                );
            }
        }
        info!(
            epoch = self.epoch_state.epoch,
            my_addr = self.my_addr.to_hex().as_str(),
            "[BatchEncryptionDKG] Manager finished."
        );
    }

    fn observe(&self) -> Result<()> {
        debug!("[BatchEncryptionDKG] state={:?}", self.state);
        Ok(())
    }

    async fn process_close_cmd(&mut self, ack_tx: Option<oneshot::Sender<()>>) -> Result<()> {
        self.stopped = true;
        if let Some(tx) = ack_tx {
            let _ = tx.send(());
        }
        Ok(())
    }

    async fn process_peer_rpc_msg(
        &mut self,
        req: (AccountAddress, crate::network::IncomingRpcRequest),
    ) -> Result<()> {
        let (peer, req) = req;
        ensure!(
            req.msg.epoch() == self.epoch_state.epoch,
            "[BatchEncryptionDKG] msg not for current epoch"
        );

        let response = match &self.state {
            InnerState::Finished { my_transcript, .. } => {
                Ok(DKGMessage::TranscriptResponse(my_transcript.clone()))
            },
            _ => bail!("[BatchEncryptionDKG] unexpected state for request"),
        };

        req.response_sender.send(response);
        Ok(())
    }

    pub async fn start_dkg(
        &mut self,
        start_time_us: u64,
        config: BatchEncryptionDKGConfig,
    ) -> Result<()> {
        ensure!(
            matches!(&self.state, InnerState::NotStarted),
            "[BatchEncryptionDKG] transcript already dealt"
        );

        let dkg_start_time = Duration::from_micros(start_time_us);
        info!(
            epoch = self.epoch_state.epoch,
            my_addr = self.my_addr,
            "[BatchEncryptionDKG] Deal transcript started."
        );

        let verifier = Arc::new(self.epoch_state.verifier.clone());
        let pub_params = BatchEncryptionDKGPublicParams::new(config.clone(), verifier);

        let mut rng = if cfg!(feature = "smoke-test") {
            StdRng::from_seed(self.my_addr.into_bytes())
        } else {
            StdRng::from_rng(thread_rng()).unwrap()
        };

        let input_secret =
            <aptos_dkg::pvss::chunky::SignedWeightedTranscript<
                aptos_batch_encryption::group::Pairing,
            > as aptos_dkg::pvss::traits::Transcript>::InputSecret::generate(&mut rng);

        let chunky_transcript =
            aptos_types::dkg::batch_encryption_dkg::BatchEncryptionDKG::generate_transcript(
                &mut rng,
                &pub_params,
                &input_secret,
                self.my_index,
                &self.dealer_sk,
                &self.dealer_pk,
            );

        let my_transcript = BatchEncryptionDKGTranscript::new(
            self.epoch_state.epoch,
            self.my_addr,
            bcs::to_bytes(&chunky_transcript)
                .map_err(|e| anyhow!("[BatchEncryptionDKG] transcript serialization error: {e}"))?,
        );

        let (agg_trx_tx, _agg_trx_rx) = aptos_channel::new(QueueStyle::KLAST, 1, None);
        self.agg_trx_tx = Some(agg_trx_tx);

        let abort_handle = self
            .agg_trx_producer
            .start_produce(
                dkg_start_time,
                self.my_addr,
                self.epoch_state.clone(),
                config,
                self.agg_trx_tx.clone(),
            )
            .await;

        self.state = InnerState::InProgress {
            start_time: dkg_start_time,
            my_transcript,
            abort_handle,
        };

        Ok(())
    }
}
