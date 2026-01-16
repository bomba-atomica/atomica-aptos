// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_crypto_derive::CryptoHasher;
use aptos_enum_conversion_derive::EnumConversion;
use aptos_reliable_broadcast::RBMessage;
pub use aptos_types::dkg::DKGTranscript;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, CryptoHasher, Debug, PartialEq)]
pub struct DKGTranscriptRequest {
    dealer_epoch: u64,
}

impl DKGTranscriptRequest {
    pub fn new(epoch: u64) -> Self {
        Self {
            dealer_epoch: epoch,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, EnumConversion, PartialEq)]
pub enum DKGMessage {
    TranscriptRequest(DKGTranscriptRequest),
    TranscriptResponse(DKGTranscript),
}

impl DKGMessage {
    pub fn epoch(&self) -> u64 {
        match self {
            DKGMessage::TranscriptRequest(request) => request.dealer_epoch,
            DKGMessage::TranscriptResponse(response) => response.metadata.epoch,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            DKGMessage::TranscriptRequest(_) => "DKGTranscriptRequest",
            DKGMessage::TranscriptResponse(_) => "DKGTranscriptResponse",
        }
    }
}

impl RBMessage for DKGMessage {}

use aptos_types::dkg::batch_encryption_dkg::{
    BatchEncryptionDKGTranscript, BatchEncryptionDKGTranscriptRequest,
};

#[derive(Clone, Serialize, Deserialize, Debug, EnumConversion, PartialEq)]
pub enum BatchEncryptionDKGMessage {
    TranscriptRequest(BatchEncryptionDKGTranscriptRequest),
    TranscriptResponse(BatchEncryptionDKGTranscript),
}

impl BatchEncryptionDKGMessage {
    pub fn epoch(&self) -> u64 {
        match self {
            BatchEncryptionDKGMessage::TranscriptRequest(request) => request.epoch,
            BatchEncryptionDKGMessage::TranscriptResponse(response) => response.epoch,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            BatchEncryptionDKGMessage::TranscriptRequest(_) => {
                "BatchEncryptionDKGTranscriptRequest"
            },
            BatchEncryptionDKGMessage::TranscriptResponse(_) => {
                "BatchEncryptionDKGTranscriptResponse"
            },
        }
    }
}

impl RBMessage for BatchEncryptionDKGMessage {}
