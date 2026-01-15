// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Batch Encryption DKG - Separate DKG for generating batch encryption keys using chunky PVSS
//!
//! This module provides structures for a DKG that runs alongside the randomness DKG to generate
//! keys for the encrypted transactions feature. It uses chunky PVSS (HKZG) instead
//! of DAS PVSS used for randomness.

use crate::on_chain_config::OnChainConfig;
use crate::validator_verifier::ValidatorConsensusInfo;
use anyhow::{Context, Error};
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};

pub type ChunkTranscript =
    aptos_dkg::pvss::chunky::SignedWeightedTranscript<aptos_batch_encryption::group::Pairing>;
pub type ChunkPP = <ChunkTranscript as aptos_dkg::pvss::traits::Transcript>::PublicParameters;
pub type ChunkEncryptPubKey =
    <ChunkTranscript as aptos_dkg::pvss::traits::Transcript>::EncryptPubKey;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchEncryptionDKGConfig {
    pub epoch: u64,
    pub threshold_weight: usize,
    pub total_weight: usize,
    pub pp: ChunkPP,
    pub eks: Vec<ChunkEncryptPubKey>,
}

impl BatchEncryptionDKGConfig {
    pub fn new(
        epoch: u64,
        threshold_weight: usize,
        total_weight: usize,
        pp: ChunkPP,
        eks: Vec<ChunkEncryptPubKey>,
    ) -> Self {
        Self {
            epoch,
            threshold_weight,
            total_weight,
            pp,
            eks,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchEncryptionDKGTranscript {
    pub epoch: u64,
    pub author: AccountAddress,
    #[serde(with = "serde_bytes")]
    pub transcript_bytes: Vec<u8>,
}

impl BatchEncryptionDKGTranscript {
    pub fn new(epoch: u64, author: AccountAddress, transcript_bytes: Vec<u8>) -> Self {
        Self {
            epoch,
            author,
            transcript_bytes,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BatchEncryptionDKGState {
    pub last_completed: Option<BatchEncryptionDKGTranscript>,
}

impl BatchEncryptionDKGState {
    pub fn last_completed_transcript(&self) -> Option<&BatchEncryptionDKGTranscript> {
        self.last_completed.as_ref()
    }
}

impl OnChainConfig for BatchEncryptionDKGState {
    const MODULE_IDENTIFIER: &'static str = "batch_encryption_dkg";
    const TYPE_IDENTIFIER: &'static str = "BatchEncryptionDKGState";
}
