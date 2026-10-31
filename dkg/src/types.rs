// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub use crate::chunky::types::{
    ChunkyDKGSubtranscriptSignatureRequest, ChunkyDKGSubtranscriptSignatureResponse,
    ChunkyDKGTranscriptRequest, MissingTranscriptRequest, MissingTranscriptResponse,
    ValidatedAggregatedTranscript,
};
use aptos_crypto_derive::CryptoHasher;
use aptos_enum_conversion_derive::EnumConversion;
use aptos_reliable_broadcast::RBMessage;
use aptos_types::dkg::timelock_dkg::ChunkyDKGSessionKind;
pub use aptos_types::dkg::{chunky_dkg::ChunkyDKGTranscript, DKGTranscript, DKGTranscriptMetadata};
use serde::{Deserialize, Serialize};

/// Once DKG starts, a validator should send this message to peers in order to collect DKG transcripts from peers.
/// This is the request for the Das DKG implementation
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

/// The DKG network message.
#[derive(Clone, Serialize, Deserialize, Debug, EnumConversion, PartialEq)]
pub enum DKGMessage {
    TranscriptRequest(DKGTranscriptRequest),
    TranscriptResponse(DKGTranscript),
    ChunkyTranscriptRequest(ChunkyDKGTranscriptRequest),
    ChunkyTranscriptResponse(ChunkyDKGTranscript),
    SubtranscriptSignatureRequest(ChunkyDKGSubtranscriptSignatureRequest),
    SubtranscriptSignatureResponse(ChunkyDKGSubtranscriptSignatureResponse),
    MissingTranscriptRequest(MissingTranscriptRequest),
    MissingTranscriptResponse(MissingTranscriptResponse),
    /// A request of the timelock chunky DKG session. Appended at the end, so the BCS encoding of every
    /// mempool message is unchanged. Responses are shared with the mempool session: a response answers
    /// the request it was sent for.
    TimelockRequest(TimelockDKGRequest),
}

/// The requests of the timelock session. Same payloads as the mempool requests, in their own enum so the
/// receiver can route them to the timelock manager.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum TimelockDKGRequest {
    Transcript(ChunkyDKGTranscriptRequest),
    SubtranscriptSignature(ChunkyDKGSubtranscriptSignatureRequest),
    MissingTranscript(MissingTranscriptRequest),
}

impl TimelockDKGRequest {
    /// The same request as a plain message, for the shared handlers.
    pub fn into_plain(self) -> DKGMessage {
        match self {
            TimelockDKGRequest::Transcript(r) => DKGMessage::ChunkyTranscriptRequest(r),
            TimelockDKGRequest::SubtranscriptSignature(r) => {
                DKGMessage::SubtranscriptSignatureRequest(r)
            },
            TimelockDKGRequest::MissingTranscript(r) => DKGMessage::MissingTranscriptRequest(r),
        }
    }

    pub fn dealer_epoch(&self) -> u64 {
        match self {
            TimelockDKGRequest::Transcript(r) => r.dealer_epoch,
            TimelockDKGRequest::SubtranscriptSignature(r) => r.dealer_epoch,
            TimelockDKGRequest::MissingTranscript(r) => r.dealer_epoch,
        }
    }
}

macro_rules! timelock_request_newtype {
    ($name:ident, $inner:ty, $variant:ident) => {
        /// The timelock-session form of a mempool request. Converts to
        /// `DKGMessage::TimelockRequest`.
        #[derive(Clone, Debug, PartialEq)]
        pub struct $name(pub $inner);

        impl From<$name> for DKGMessage {
            fn from(value: $name) -> Self {
                DKGMessage::TimelockRequest(TimelockDKGRequest::$variant(value.0))
            }
        }

        impl TryFrom<DKGMessage> for $name {
            type Error = anyhow::Error;

            fn try_from(msg: DKGMessage) -> Result<Self, Self::Error> {
                match msg {
                    DKGMessage::TimelockRequest(TimelockDKGRequest::$variant(r)) => Ok(Self(r)),
                    other => Err(anyhow::anyhow!(
                        "unexpected message {} for {}",
                        other.name(),
                        stringify!($name)
                    )),
                }
            }
        }
    };
}

timelock_request_newtype!(
    TimelockTranscriptRequest,
    ChunkyDKGTranscriptRequest,
    Transcript
);
timelock_request_newtype!(
    TimelockSignatureRequest,
    ChunkyDKGSubtranscriptSignatureRequest,
    SubtranscriptSignature
);
timelock_request_newtype!(
    TimelockMissingTranscriptRequest,
    MissingTranscriptRequest,
    MissingTranscript
);

impl DKGMessage {
    pub fn epoch(&self) -> u64 {
        match self {
            DKGMessage::TranscriptRequest(request) => request.dealer_epoch,
            DKGMessage::TranscriptResponse(response) => response.metadata.epoch,
            DKGMessage::ChunkyTranscriptRequest(request) => request.dealer_epoch,
            DKGMessage::ChunkyTranscriptResponse(response) => response.metadata.epoch,
            DKGMessage::SubtranscriptSignatureRequest(request) => request.dealer_epoch,
            DKGMessage::SubtranscriptSignatureResponse(response) => response.dealer_epoch,
            DKGMessage::MissingTranscriptRequest(request) => request.dealer_epoch,
            DKGMessage::MissingTranscriptResponse(response) => response.transcript.metadata.epoch,
            DKGMessage::TimelockRequest(request) => request.dealer_epoch(),
        }
    }

    /// Which session a request belongs to, with the request as a plain message. Every message that is
    /// not a `TimelockRequest` belongs to the mempool session.
    pub fn split_session_kind(self) -> (ChunkyDKGSessionKind, DKGMessage) {
        match self {
            DKGMessage::TimelockRequest(request) => {
                (ChunkyDKGSessionKind::Timelock, request.into_plain())
            },
            other => (ChunkyDKGSessionKind::Mempool, other),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            DKGMessage::TranscriptRequest(_) => "DKGTranscriptRequest",
            DKGMessage::TranscriptResponse(_) => "DKGTranscriptResponse",
            DKGMessage::ChunkyTranscriptRequest(_) => "ChunkyDKGTranscriptRequest",
            DKGMessage::ChunkyTranscriptResponse(_) => "ChunkyDKGTranscriptResponse",
            DKGMessage::SubtranscriptSignatureRequest(_) => {
                "ChunkyDKGSubtranscriptSignatureRequest"
            },
            DKGMessage::SubtranscriptSignatureResponse(_) => {
                "ChunkyDKGSubtranscriptSignatureResponse"
            },
            DKGMessage::MissingTranscriptRequest(_) => "MissingTranscriptRequest",
            DKGMessage::MissingTranscriptResponse(_) => "MissingTranscriptResponse",
            DKGMessage::TimelockRequest(_) => "TimelockRequest",
        }
    }
}

impl RBMessage for DKGMessage {}
