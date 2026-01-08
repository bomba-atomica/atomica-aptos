// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use self::real_dkg::RealDKG;
use crate::{
    contract_event::ContractEvent,
    dkg::real_dkg::{rounding::DKGRoundingProfile, Transcripts},
    on_chain_config::{OnChainConfig, OnChainRandomnessConfig, RandomnessConfigMoveStruct},
    validator_verifier::{
        ValidatorConsensusInfo, ValidatorConsensusInfoMoveStruct, ValidatorVerifier,
    },
};
use anyhow::{bail, Context, Result};
use aptos_crypto::Uniform;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use move_core_types::{
    account_address::AccountAddress, ident_str, identifier::IdentStr, language_storage::TypeTag,
    move_resource::MoveStructType,
};
use once_cell::sync::Lazy;
use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fmt::{Debug, Formatter},
    time::Duration,
};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, CryptoHasher, BCSCryptoHash)]
pub struct DKGTranscriptMetadata {
    pub epoch: u64,
    pub author: AccountAddress,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DKGStartEvent {
    pub session_metadata: DKGSessionMetadata,
    pub start_time_us: u64,
}

impl MoveStructType for DKGStartEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("dkg");
    const STRUCT_NAME: &'static IdentStr = ident_str!("DKGStartEvent");
}

pub static DKG_START_EVENT_MOVE_TYPE_TAG: Lazy<TypeTag> =
    Lazy::new(|| TypeTag::Struct(Box::new(DKGStartEvent::struct_tag())));

/// DKG transcript and its metadata.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DKGTranscript {
    pub metadata: DKGTranscriptMetadata,
    #[serde(with = "serde_bytes")]
    pub transcript_bytes: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub mpk_bytes: Vec<u8>, // Extracted MPK (G2 compressed, 96 bytes) for timelock
}

impl Debug for DKGTranscript {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DKGTranscript")
            .field("metadata", &self.metadata)
            .field("transcript_bytes_len", &self.transcript_bytes.len())
            .field("mpk_bytes_len", &self.mpk_bytes.len())
            .finish()
    }
}

impl DKGTranscript {
    pub fn new(
        epoch: u64,
        author: AccountAddress,
        transcript_bytes: Vec<u8>,
        mpk_bytes: Vec<u8>,
    ) -> Self {
        Self {
            metadata: DKGTranscriptMetadata { epoch, author },
            transcript_bytes,
            mpk_bytes,
        }
    }

    pub fn dummy() -> Self {
        Self {
            metadata: DKGTranscriptMetadata {
                epoch: 0,
                author: AccountAddress::ZERO,
            },
            transcript_bytes: vec![],
            mpk_bytes: vec![],
        }
    }

    pub(crate) fn verify(&self, verifier: &ValidatorVerifier) -> Result<()> {
        let transcripts: Transcripts = bcs::from_bytes(&self.transcript_bytes)
            .context("Transcripts deserialization failed")?;
        RealDKG::verify_transcript_extra(&transcripts, verifier, true, None)
    }
}

/// Reflection of `0x1::dkg::DKGSessionMetadata` in rust.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DKGSessionMetadata {
    pub dealer_epoch: u64,
    pub randomness_config: RandomnessConfigMoveStruct,
    pub dealer_validator_set: Vec<ValidatorConsensusInfoMoveStruct>,
    pub target_validator_set: Vec<ValidatorConsensusInfoMoveStruct>,
}

impl DKGSessionMetadata {
    pub fn target_validator_consensus_infos_cloned(&self) -> Vec<ValidatorConsensusInfo> {
        self.target_validator_set
            .clone()
            .into_iter()
            .filter_map(|obj| obj.try_into().ok())
            .collect()
    }

    pub fn dealer_consensus_infos_cloned(&self) -> Vec<ValidatorConsensusInfo> {
        self.dealer_validator_set
            .clone()
            .into_iter()
            .filter_map(|obj| obj.try_into().ok())
            .collect()
    }

    pub fn randomness_config_derived(&self) -> Option<OnChainRandomnessConfig> {
        OnChainRandomnessConfig::try_from(self.randomness_config.clone()).ok()
    }
}

impl MayHaveRoundingSummary for DKGSessionMetadata {
    fn rounding_summary(&self) -> Option<&RoundingSummary> {
        None
    }
}

/// Reflection of Move type `0x1::dkg::DKGSessionState`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DKGSessionState {
    pub metadata: DKGSessionMetadata,
    pub start_time_us: u64,
    pub transcript: Vec<u8>,
}

impl DKGSessionState {
    pub fn target_epoch(&self) -> u64 {
        self.metadata.dealer_epoch + 1
    }
}
/// Reflection of Move type `0x1::dkg::DKGState`.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct DKGState {
    pub last_completed: Option<DKGSessionState>,
    pub in_progress: Option<DKGSessionState>,
}

impl DKGState {
    pub fn maybe_last_complete(&self, epoch: u64) -> Option<&DKGSessionState> {
        match &self.last_completed {
            Some(session) if session.target_epoch() == epoch => Some(session),
            _ => None,
        }
    }

    pub fn last_complete(&self) -> &DKGSessionState {
        self.last_completed.as_ref().unwrap()
    }
}

impl OnChainConfig for DKGState {
    const MODULE_IDENTIFIER: &'static str = "dkg";
    const TYPE_IDENTIFIER: &'static str = "DKGState";
}

#[derive(Clone, Debug, Default)]
pub struct RoundingSummary {
    pub method: String,
    pub output: DKGRoundingProfile,
    pub error: Option<String>,
    pub exec_time: Duration,
}

pub trait MayHaveRoundingSummary {
    fn rounding_summary(&self) -> Option<&RoundingSummary>;
}

/// NOTE: this is a subset of the full scheme. Some data items/algorithms are not used in DKG and are omitted.
pub trait DKGTrait: Debug {
    type DealerPrivateKey;
    type PublicParams: Clone + Debug + Send + Sync + MayHaveRoundingSummary;
    type Transcript: Clone + Send + Sync + Serialize + for<'a> Deserialize<'a>;
    type InputSecret: Uniform;
    type DealtSecret;
    type DealtSecretShare;
    type DealtPubKeyShare;
    type NewValidatorDecryptKey: Uniform;

    fn new_public_params(dkg_session_metadata: &DKGSessionMetadata) -> Result<Self::PublicParams>;
    fn aggregate_input_secret(secrets: Vec<Self::InputSecret>) -> Self::InputSecret;
    fn dealt_secret_from_input(
        pub_params: &Self::PublicParams,
        input: &Self::InputSecret,
    ) -> Self::DealtSecret;
    fn generate_transcript<R: CryptoRng + RngCore>(
        rng: &mut R,
        params: &Self::PublicParams,
        input_secret: &Self::InputSecret,
        my_index: u64,
        sk: &Self::DealerPrivateKey,
    ) -> Self::Transcript;

    /// NOTE: used in VM.
    fn verify_transcript(params: &Self::PublicParams, trx: &Self::Transcript) -> Result<()>;

    fn verify_transcript_extra(
        trx: &Self::Transcript,
        verifier: &ValidatorVerifier,
        checks_voting_power: bool,
        ensures_single_dealer: Option<AccountAddress>,
    ) -> Result<()>;

    fn aggregate_transcripts(
        params: &Self::PublicParams,
        accumulator: &mut Self::Transcript,
        element: Self::Transcript,
    );

    fn decrypt_secret_share_from_transcript(
        pub_params: &Self::PublicParams,
        trx: &Self::Transcript,
        player_idx: u64,
        dk: &Self::NewValidatorDecryptKey,
    ) -> Result<(Self::DealtSecretShare, Self::DealtPubKeyShare)>;

    fn reconstruct_secret_from_shares(
        pub_params: &Self::PublicParams,
        player_share_pairs: Vec<(u64, Self::DealtSecretShare)>,
    ) -> Result<Self::DealtSecret>;
    fn get_dealers(transcript: &Self::Transcript) -> BTreeSet<u64>;

    /// Check if public params are properly configured for DKG (encryption keys match player count)
    fn is_valid_for_dkg(pub_params: &Self::PublicParams) -> bool;
}

pub mod dummy_dkg;
pub mod ibe_dkg;
pub mod real_dkg;

pub type DefaultDKG = RealDKG;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct DecryptionKeyShare {
    pub timelock_id: u64,
    pub author: AccountAddress,
    pub validator_idx: u64, // Validator's index in DKG participant set (for Lagrange weights)
    pub share: Vec<u8>,     // G1 point: s_i × Q_id
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct TimelockConfig {
    pub threshold: u64,
    pub total_validators: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StartKeyGenEvent {
    pub epoch: u64,
    pub config: TimelockConfig,
}

impl MoveStructType for StartKeyGenEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("timelock");
    const STRUCT_NAME: &'static IdentStr = ident_str!("StartKeyGenEvent");
}

impl TryFrom<&ContractEvent> for StartKeyGenEvent {
    type Error = anyhow::Error;

    fn try_from(event: &ContractEvent) -> Result<Self> {
        if event.type_tag() != &TypeTag::Struct(Box::new(Self::struct_tag())) {
            bail!("Expected StartKeyGenEvent tag");
        }
        bcs::from_bytes(event.event_data()).context("Failed to deserialize StartKeyGenEvent")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RequestRevealEvent {
    pub deadline: u64,
    pub timelock_ids: Vec<u64>,
}

impl MoveStructType for RequestRevealEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("timelock");
    const STRUCT_NAME: &'static IdentStr = ident_str!("RequestRevealEvent");
}

impl TryFrom<&ContractEvent> for RequestRevealEvent {
    type Error = anyhow::Error;

    fn try_from(event: &ContractEvent) -> Result<Self> {
        if event.type_tag() != &TypeTag::Struct(Box::new(Self::struct_tag())) {
            bail!("Expected RequestRevealEvent tag");
        }
        bcs::from_bytes(event.event_data()).context("Failed to deserialize RequestRevealEvent")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimelockRegisteredEvent {
    pub timelock_id: u64,
    pub deadline: u64,
}

impl MoveStructType for TimelockRegisteredEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("timelock");
    const STRUCT_NAME: &'static IdentStr = ident_str!("TimelockRegisteredEvent");
}

impl TryFrom<&ContractEvent> for TimelockRegisteredEvent {
    type Error = anyhow::Error;

    fn try_from(event: &ContractEvent) -> Result<Self> {
        if event.type_tag() != &TypeTag::Struct(Box::new(Self::struct_tag())) {
            bail!("Expected TimelockRegisteredEvent tag");
        }
        bcs::from_bytes(event.event_data()).context("Failed to deserialize TimelockRegisteredEvent")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MasterPublicKeyPublishedEvent {
    pub id: u64,
    pub master_public_key: Vec<u8>,
}

impl MoveStructType for MasterPublicKeyPublishedEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("threshold_dsa");
    const STRUCT_NAME: &'static IdentStr = ident_str!("MasterPublicKeyPublishedEvent");
}

impl TryFrom<&ContractEvent> for MasterPublicKeyPublishedEvent {
    type Error = anyhow::Error;

    fn try_from(event: &ContractEvent) -> Result<Self> {
        if event.type_tag() != &TypeTag::Struct(Box::new(Self::struct_tag())) {
            bail!("Expected MasterPublicKeyPublishedEvent tag");
        }
        bcs::from_bytes(event.event_data())
            .context("Failed to deserialize MasterPublicKeyPublishedEvent")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecretRevealedEvent {
    pub timelock_id: u64,
    pub deadline: u64,
    pub secret: Vec<u8>,
}

impl MoveStructType for SecretRevealedEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("timelock");
    const STRUCT_NAME: &'static IdentStr = ident_str!("SecretRevealedEvent");
}

impl TryFrom<&ContractEvent> for SecretRevealedEvent {
    type Error = anyhow::Error;

    fn try_from(event: &ContractEvent) -> Result<Self> {
        if event.type_tag() != &TypeTag::Struct(Box::new(Self::struct_tag())) {
            bail!("Expected SecretRevealedEvent tag");
        }
        bcs::from_bytes(event.event_data()).context("Failed to deserialize SecretRevealedEvent")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decryption_key_share_bcs() {
        let share = DecryptionKeyShare {
            timelock_id: 100,
            author: AccountAddress::ONE,
            validator_idx: 0,
            share: vec![1, 2, 3, 4],
        };
        let bytes = bcs::to_bytes(&share).expect("serialization failed");
        let decoded: DecryptionKeyShare = bcs::from_bytes(&bytes).expect("deserialization failed");
        assert_eq!(share, decoded);
    }
}
