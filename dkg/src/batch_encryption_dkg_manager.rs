// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Batch Encryption DKG Manager
//!
//! This module handles the DKG protocol for generating batch encryption keys.
//! It runs alongside the randomness DKG and uses chunky PVSS (HKZG) for threshold
//! encryption key generation.
//!
//! Note: This manager handles transcript generation. Transcript aggregation is
//! done via reliable broadcast in the main DKG flow.

use crate::counters::DKG_STAGE_SECONDS;
use anyhow::{anyhow, ensure, Result};
use aptos_crypto::{bls12381, Uniform};
use aptos_infallible::duration_since_epoch;
use aptos_logger::{debug, error, info, warn};
use futures::{FutureExt, StreamExt};
use rand::{prelude::StdRng, thread_rng, SeedableRng};
use std::{sync::Arc, time::Duration};

use aptos_types::{
    account_address::AccountAddress,
    dkg::{
        batch_encryption_dkg::{
            BatchEncryptionDKG, BatchEncryptionDKGConfig, BatchEncryptionDKGPublicParams,
            BatchEncryptionDKGTranscript,
        },
        DKGStartEvent,
    },
    epoch_state::EpochState,
    validator_verifier::ValidatorConsensusInfo,
};

#[derive(Clone, Debug)]
pub enum BatchEncryptionDKGStateKind {
    NotStarted,
    InProgress {
        start_time: Duration,
        my_transcript: BatchEncryptionDKGTranscript,
    },
    Finished {
        start_time: Duration,
        my_transcript: BatchEncryptionDKGTranscript,
        config: BatchEncryptionDKGConfig,
    },
}

impl Default for BatchEncryptionDKGStateKind {
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
    state: BatchEncryptionDKGStateKind,
    config: Option<BatchEncryptionDKGConfig>,
}

impl BatchEncryptionDKGManager {
    pub fn new(
        my_addr: AccountAddress,
        my_index: usize,
        epoch_state: Arc<EpochState>,
        dealer_sk: Arc<bls12381::PrivateKey>,
        dealer_pk: Arc<bls12381::PublicKey>,
    ) -> Self {
        Self {
            my_addr,
            my_index,
            epoch_state,
            dealer_sk,
            dealer_pk,
            state: BatchEncryptionDKGStateKind::NotStarted,
            config: None,
        }
    }

    pub fn create_config(
        epoch: u64,
        next_validators: &[ValidatorConsensusInfo],
        max_batch_size: usize,
        number_of_rounds: usize,
        rng: &mut (impl rand::CryptoRng + rand::RngCore),
    ) -> BatchEncryptionDKGConfig {
        BatchEncryptionDKGConfig::new_for_epoch(
            epoch,
            next_validators,
            max_batch_size,
            number_of_rounds,
            rng,
        )
    }

    pub fn epoch(&self) -> u64 {
        self.epoch_state.epoch
    }

    pub fn state(&self) -> &BatchEncryptionDKGStateKind {
        &self.state
    }

    pub fn set_state(&mut self, state: BatchEncryptionDKGStateKind) {
        self.state = state;
    }

    pub fn observe(&self) {
        debug!("[BatchEncryptionDKG] state={:?}", self.state);
    }

    pub fn my_transcript(&self) -> Option<&BatchEncryptionDKGTranscript> {
        match &self.state {
            BatchEncryptionDKGStateKind::Finished { my_transcript, .. } => Some(my_transcript),
            _ => None,
        }
    }

    pub fn generate_transcript(
        &self,
        config: &BatchEncryptionDKGConfig,
    ) -> Result<BatchEncryptionDKGTranscript> {
        let verifier = self.epoch_state.verifier.clone();
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

        let chunky_transcript = BatchEncryptionDKG::generate_transcript(
            &mut rng,
            &pub_params,
            &input_secret,
            self.my_index,
            &self.dealer_sk,
            &self.dealer_pk,
        );

        let transcript = BatchEncryptionDKGTranscript::new(
            self.epoch_state.epoch,
            self.my_addr,
            bcs::to_bytes(&chunky_transcript)
                .map_err(|e| anyhow!("[BatchEncryptionDKG] transcript serialization error: {e}"))?,
        );

        Ok(transcript)
    }

    pub async fn start_dkg(
        &mut self,
        start_time_us: u64,
        config: BatchEncryptionDKGConfig,
    ) -> Result<()> {
        ensure!(
            matches!(&self.state, BatchEncryptionDKGStateKind::NotStarted),
            "[BatchEncryptionDKG] transcript already dealt"
        );

        let dkg_start_time = Duration::from_micros(start_time_us);
        info!(
            epoch = self.epoch_state.epoch,
            my_addr = self.my_addr,
            "[BatchEncryptionDKG] Deal transcript started."
        );

        let my_transcript = self.generate_transcript(&config)?;

        self.config = Some(config.clone());
        self.state = BatchEncryptionDKGStateKind::InProgress {
            start_time: dkg_start_time,
            my_transcript,
        };

        Ok(())
    }

    pub fn complete_dkg(&mut self, aggregated_transcript: BatchEncryptionDKGTranscript) {
        if let (
            Some(config),
            BatchEncryptionDKGStateKind::InProgress {
                start_time,
                my_transcript,
            },
        ) = (self.config.clone(), self.state.clone())
        {
            self.state = BatchEncryptionDKGStateKind::Finished {
                start_time,
                my_transcript,
                config,
            };
            info!(
                epoch = self.epoch_state.epoch,
                my_addr = self.my_addr,
                "[BatchEncryptionDKG] DKG completed with aggregated transcript"
            );
        }
    }

    pub fn into_on_chain_state(
        self,
    ) -> Option<(u64, BatchEncryptionDKGTranscript, BatchEncryptionDKGConfig)> {
        match self.state {
            BatchEncryptionDKGStateKind::Finished {
                start_time: _,
                my_transcript,
                config,
            } => Some((self.epoch_state.epoch, my_transcript, config)),
            _ => None,
        }
    }

    pub async fn run(
        mut self,
        mut dkg_start_event_rx: aptos_channels::aptos_channel::Receiver<(), DKGStartEvent>,
        close_rx: futures_channel::oneshot::Receiver<futures_channel::oneshot::Sender<()>>,
    ) {
        info!(
            epoch = self.epoch_state.epoch,
            my_addr = self.my_addr.to_hex().as_str(),
            "[BatchEncryptionDKG] BatchEncryptionDKGManager started."
        );
        let mut interval = tokio::time::interval(Duration::from_millis(5000));
        let mut close_rx = close_rx.into_stream();

        while !matches!(self.state, BatchEncryptionDKGStateKind::Finished { .. }) {
            let handling_result = tokio::select! {
                dkg_start_event = dkg_start_event_rx.select_next_some() => {
                    self.process_dkg_start_event(dkg_start_event)
                        .await
                        .map_err(|e| anyhow!("[BatchEncryptionDKG] process_dkg_start_event failed: {e}"))
                },
                close_req = close_rx.select_next_some() => {
                    self.process_close_cmd(close_req.ok());
                    Ok(())
                },
                _ = interval.tick() => {
                    self.observe();
                    Ok(())
                },
            };

            if let Err(e) = handling_result {
                error!(
                    epoch = self.epoch_state.epoch,
                    my_addr = self.my_addr.to_hex().as_str(),
                    "[BatchEncryptionDKG] BatchEncryptionDKGManager handling error: {e}"
                );
            }
        }
        info!(
            epoch = self.epoch_state.epoch,
            my_addr = self.my_addr.to_hex().as_str(),
            "[BatchEncryptionDKG] BatchEncryptionDKGManager finished."
        );
    }

    async fn process_dkg_start_event(&mut self, event: DKGStartEvent) -> Result<()> {
        info!(
            epoch = self.epoch_state.epoch,
            my_addr = self.my_addr,
            "[BatchEncryptionDKG] Processing DKGStart event."
        );
        let DKGStartEvent {
            session_metadata,
            start_time_us,
        } = event;

        if self.epoch_state.epoch != session_metadata.dealer_epoch {
            warn!(
                "[BatchEncryptionDKG] event (from epoch {}) not for current epoch ({}), ignoring",
                session_metadata.dealer_epoch, self.epoch_state.epoch
            );
            return Ok(());
        }

        let validators: Vec<ValidatorConsensusInfo> =
            session_metadata.target_validator_consensus_infos_cloned();

        let config = BatchEncryptionDKGManager::create_config(
            session_metadata.dealer_epoch,
            &validators,
            100,
            1,
            &mut rand::thread_rng(),
        );

        self.start_dkg(start_time_us, config).await
    }

    fn process_close_cmd(&mut self, _ack_tx: Option<futures_channel::oneshot::Sender<()>>) {
        match &self.state {
            BatchEncryptionDKGStateKind::InProgress { start_time, .. } => {
                let epoch_change_time = duration_since_epoch();
                let secs_since_dkg_start =
                    epoch_change_time.as_secs_f64() - start_time.as_secs_f64();
                DKG_STAGE_SECONDS
                    .with_label_values(&[self.my_addr.to_hex().as_str(), "epoch_change"])
                    .observe(secs_since_dkg_start);
                info!(
                    epoch = self.epoch_state.epoch,
                    my_addr = self.my_addr,
                    secs_since_dkg_start = secs_since_dkg_start,
                    "[BatchEncryptionDKG] epoch change, DKG incomplete.",
                );
            },
            _ => {},
        }
    }
}
