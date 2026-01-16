// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    batch_encryption_transcript_aggregation::BatchEncryptionTranscriptAggregationState,
    types::BatchEncryptionDKGMessage,
};
use aptos_channels::aptos_channel::Sender;
use aptos_logger::info;
use aptos_reliable_broadcast::ReliableBroadcast;
use aptos_types::dkg::batch_encryption_dkg::{
    BatchEncryptionDKGPublicParams, BatchEncryptionDKGTranscript,
};
use aptos_types::epoch_state::EpochState;
use futures::future::AbortHandle;
use futures_util::future::Abortable;
use move_core_types::account_address::AccountAddress;
use std::{sync::Arc, time::Duration};
use tokio_retry::strategy::ExponentialBackoff;

pub trait TBatchEncryptionAggTranscriptProducer: Send + Sync {
    fn start_produce(
        &self,
        start_time: Duration,
        my_addr: AccountAddress,
        epoch_state: Arc<EpochState>,
        dkg_config: BatchEncryptionDKGPublicParams,
        agg_trx_tx: Option<Sender<(), BatchEncryptionDKGTranscript>>,
    ) -> AbortHandle;
}

pub struct BatchEncryptionAggTranscriptProducer {
    reliable_broadcast: Arc<ReliableBroadcast<BatchEncryptionDKGMessage, ExponentialBackoff>>,
}

impl BatchEncryptionAggTranscriptProducer {
    pub fn new(
        reliable_broadcast: ReliableBroadcast<BatchEncryptionDKGMessage, ExponentialBackoff>,
    ) -> Self {
        Self {
            reliable_broadcast: Arc::new(reliable_broadcast),
        }
    }
}

impl TBatchEncryptionAggTranscriptProducer for BatchEncryptionAggTranscriptProducer {
    fn start_produce(
        &self,
        start_time: Duration,
        my_addr: AccountAddress,
        epoch_state: Arc<EpochState>,
        params: BatchEncryptionDKGPublicParams,
        agg_trx_tx: Option<Sender<(), BatchEncryptionDKGTranscript>>,
    ) -> AbortHandle {
        let epoch = epoch_state.epoch;
        let rb = self.reliable_broadcast.clone();
        let req = params.get_transcript_request();
        let agg_state = Arc::new(BatchEncryptionTranscriptAggregationState::new(
            start_time, my_addr, params, epoch,
        ));
        let task = async move {
            let agg_trx = rb
                .broadcast(req, agg_state)
                .await
                .expect("batch encryption broadcast cannot fail");
            info!(
                epoch = epoch,
                my_addr = my_addr,
                "[BatchEncryptionDKG] aggregated transcript locally"
            );
            if let Err(e) = agg_trx_tx
                .expect("[BatchEncryptionDKG] agg_trx_tx should be available")
                .push((), agg_trx)
            {
                info!(
                    epoch = epoch,
                    my_addr = my_addr,
                    "[BatchEncryptionDKG] Failed to send aggregated transcript: {:?}",
                    e
                );
            }
        };
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        tokio::spawn(Abortable::new(task, abort_registration));
        abort_handle
    }
}

#[cfg(test)]
pub struct BatchEncryptionDummyAggTranscriptProducer {}

#[cfg(test)]
impl TBatchEncryptionAggTranscriptProducer for BatchEncryptionDummyAggTranscriptProducer {
    fn start_produce(
        &self,
        _start_time: Duration,
        _my_addr: AccountAddress,
        _epoch_state: Arc<EpochState>,
        _dkg_config: BatchEncryptionDKGPublicParams,
        _agg_trx_tx: Option<Sender<(), BatchEncryptionDKGTranscript>>,
    ) -> AbortHandle {
        let (abort_handle, _) = AbortHandle::new_pair();
        abort_handle
    }
}
