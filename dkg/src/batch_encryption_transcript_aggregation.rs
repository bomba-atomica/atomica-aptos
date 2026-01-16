// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::counters::DKG_STAGE_SECONDS;
use crate::types::BatchEncryptionDKGMessage;
use anyhow::{ensure, Context};
use aptos_consensus_types::common::Author;
use aptos_dkg::pvss::traits::transcript::HasAggregatableSubtranscript;
use aptos_infallible::{duration_since_epoch, Mutex};
use aptos_logger::info;
use aptos_reliable_broadcast::BroadcastStatus;
use aptos_types::dkg::batch_encryption_dkg::{
    BatchEncryptionDKG, BatchEncryptionDKGPublicParams, BatchEncryptionDKGTranscript,
    BatchEncryptionDKGTranscriptRequest,
};
use move_core_types::account_address::AccountAddress;
use std::{collections::HashSet, sync::Arc, time::Duration};

pub struct BatchEncryptionTranscriptAggregator {
    pub contributors: HashSet<AccountAddress>,
    pub trx: Option<aptos_types::dkg::batch_encryption_dkg::ChunkTranscript>,
}

impl Default for BatchEncryptionTranscriptAggregator {
    fn default() -> Self {
        Self {
            contributors: HashSet::new(),
            trx: None,
        }
    }
}

pub struct BatchEncryptionTranscriptAggregationState {
    start_time: Duration,
    my_addr: AccountAddress,
    valid_peer_transcript_seen: std::sync::atomic::AtomicBool,
    trx_aggregator: Arc<Mutex<BatchEncryptionTranscriptAggregator>>,
    dkg_pub_params: BatchEncryptionDKGPublicParams,
    epoch: u64,
}

impl BatchEncryptionTranscriptAggregationState {
    pub fn new(
        start_time: Duration,
        my_addr: AccountAddress,
        dkg_pub_params: BatchEncryptionDKGPublicParams,
        epoch: u64,
    ) -> Self {
        Self {
            start_time,
            my_addr,
            valid_peer_transcript_seen: std::sync::atomic::AtomicBool::new(false),
            trx_aggregator: Arc::new(Mutex::new(BatchEncryptionTranscriptAggregator::default())),
            dkg_pub_params,
            epoch,
        }
    }
}

impl BroadcastStatus<BatchEncryptionDKGMessage> for Arc<BatchEncryptionTranscriptAggregationState> {
    type Aggregated = BatchEncryptionDKGTranscript;
    type Message = BatchEncryptionDKGTranscriptRequest;
    type Response = BatchEncryptionDKGTranscript;

    fn add(
        &self,
        sender: Author,
        response: Self::Response,
    ) -> anyhow::Result<Option<Self::Aggregated>> {
        ensure!(
            response.epoch == self.epoch,
            "[BatchEncryptionDKG] adding peer transcript failed with invalid epoch"
        );

        let peer_power = self.dkg_pub_params.verifier.get_voting_power(&sender);
        ensure!(
            peer_power.is_some(),
            "[BatchEncryptionDKG] adding peer transcript failed with illegal dealer"
        );
        ensure!(
            response.author == sender,
            "[BatchEncryptionDKG] adding peer transcript failed with node author mismatch"
        );

        let transcript = response
            .deserialize()
            .context("Failed to deserialize transcript")?;

        BatchEncryptionDKG::verify_transcript(&transcript, &self.dkg_pub_params, sender)
            .context("Batch encryption transcript verification failed")?;

        let mut trx_aggregator = self.trx_aggregator.lock();
        if trx_aggregator.contributors.contains(&response.author) {
            return Ok(None);
        }

        let is_self = self.my_addr == sender;
        if !is_self
            && !self
                .valid_peer_transcript_seen
                .load(std::sync::atomic::Ordering::Relaxed)
        {
            let secs_since_dkg_start =
                duration_since_epoch().as_secs_f64() - self.start_time.as_secs_f64();
            self.valid_peer_transcript_seen
                .store(true, std::sync::atomic::Ordering::Relaxed);
            DKG_STAGE_SECONDS
                .with_label_values(&[
                    self.my_addr.to_hex().as_str(),
                    "first_valid_peer_transcript",
                ])
                .observe(secs_since_dkg_start);
            info!(
                epoch = self.epoch,
                peer = sender,
                "[BatchEncryptionDKG] first valid peer transcript received after {} secs",
                secs_since_dkg_start
            );
        }

        trx_aggregator.contributors.insert(response.author);

        if let Some(agg_trx) = trx_aggregator.trx.as_mut() {
            let mut subtranscript = agg_trx.get_subtranscript();
            BatchEncryptionDKG::aggregate_transcripts(
                &mut subtranscript,
                &transcript,
                &self.dkg_pub_params.config,
            )
            .context("Failed to aggregate transcripts")?;
        } else {
            trx_aggregator.trx = Some(transcript);
        }

        let threshold = self.dkg_pub_params.verifier.quorum_voting_power();
        let power_check_result = self
            .dkg_pub_params
            .verifier
            .check_voting_power(trx_aggregator.contributors.iter(), true);

        let new_total_power = match &power_check_result {
            Ok(x) => Some(*x),
            Err(aptos_types::validator_verifier::VerifyError::TooLittleVotingPower {
                voting_power,
                ..
            }) => Some(*voting_power),
            _ => None,
        };

        let maybe_aggregated = power_check_result
            .ok()
            .map(|_| trx_aggregator.trx.clone().unwrap());

        info!(
            epoch = self.epoch,
            peer = sender,
            is_self = is_self,
            peer_power = peer_power,
            new_total_power = new_total_power,
            threshold = threshold,
            threshold_exceeded = maybe_aggregated.is_some(),
            "[BatchEncryptionDKG] added transcript from validator, {}/{} aggregated",
            new_total_power.unwrap_or(0),
            threshold
        );

        if let Some(aggregated_trx) = maybe_aggregated {
            let transcript_bytes = bcs::to_bytes(&aggregated_trx)
                .context("Failed to serialize aggregated transcript")?;
            return Ok(Some(BatchEncryptionDKGTranscript::new(
                self.epoch,
                self.my_addr,
                transcript_bytes,
            )));
        }

        Ok(None)
    }
}
