// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Batch Encryption DKG - Separate DKG for generating batch encryption keys using chunky PVSS
//!
//! This module provides structures for a DKG that runs alongside the randomness DKG to generate
//! keys for the encrypted transactions feature. It uses chunky PVSS (HKZG) instead
//! of DAS PVSS used for randomness.

use crate::{
    on_chain_config::OnChainConfig,
    validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier},
};
use anyhow::{anyhow, Context, Result};
use aptos_batch_encryption::{
    group::{Fr, G2Affine},
    schemes::fptx_weighted::FPTXWeighted,
    traits::BatchThresholdEncryption,
};
use aptos_crypto::{bls12381, weighted_config::WeightedConfigArkworks, SecretSharingConfig as _};
use aptos_dkg::pvss::{
    traits::{
        transcript::{HasAggregatableSubtranscript, Subtranscript},
        Aggregatable, Transcript,
    },
    Player,
};
use ark_ec::AffineRepr;
use move_core_types::account_address::AccountAddress;
use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[cfg(test)]
pub use self::ChunkTranscript as ChunkTranscriptType;

pub type ChunkTranscript =
    aptos_dkg::pvss::chunky::SignedWeightedTranscript<aptos_batch_encryption::group::Pairing>;
pub type ChunkPP = <ChunkTranscript as Transcript>::PublicParameters;
pub type ChunkEncryptPubKey = <ChunkTranscript as Transcript>::EncryptPubKey;
pub type ChunkSigningSecretKey = <ChunkTranscript as Transcript>::SigningSecretKey;
pub type ChunkSigningPubKey = <ChunkTranscript as Transcript>::SigningPubKey;
pub type ChunkDecryptPrivKey = <ChunkTranscript as Transcript>::DecryptPrivKey;
pub type ChunkInputSecret = <ChunkTranscript as Transcript>::InputSecret;

pub type MasterSecretKeyShare = <FPTXWeighted as BatchThresholdEncryption>::MasterSecretKeyShare;
pub type DigestKey = <FPTXWeighted as BatchThresholdEncryption>::DigestKey;
pub type EncryptionKey = <FPTXWeighted as BatchThresholdEncryption>::EncryptionKey;
pub type VerificationKey = <FPTXWeighted as BatchThresholdEncryption>::VerificationKey;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchEncryptionDKGConfig {
    pub epoch: u64,
    pub threshold_weight: usize,
    pub total_weight: usize,
    pub pp: ChunkPP,
    pub eks: Vec<ChunkEncryptPubKey>,
    pub max_batch_size: usize,
    pub number_of_rounds: usize,
}

impl BatchEncryptionDKGConfig {
    pub fn new(
        epoch: u64,
        threshold_weight: usize,
        total_weight: usize,
        pp: ChunkPP,
        eks: Vec<ChunkEncryptPubKey>,
        max_batch_size: usize,
        number_of_rounds: usize,
    ) -> Self {
        Self {
            epoch,
            threshold_weight,
            total_weight,
            pp,
            eks,
            max_batch_size,
            number_of_rounds,
        }
    }

    pub fn new_for_epoch(
        cur_epoch: u64,
        next_validators: &[ValidatorConsensusInfo],
        max_batch_size: usize,
        number_of_rounds: usize,
        rng: &mut (impl CryptoRng + RngCore),
    ) -> Self {
        let n = next_validators.len();
        let total_weight = n;
        let threshold_weight = (2 * total_weight / 3) as usize;

        let weights: Vec<usize> = (0..n).map(|_| 1).collect();
        let chunky_config: WeightedConfigArkworks<Fr> =
            WeightedConfigArkworks::new(threshold_weight, weights)
                .expect("Failed to create weighted config for batch encryption");

        let validator_consensus_keys: Vec<bls12381::PublicKey> = next_validators
            .iter()
            .map(|vi| vi.public_key.clone())
            .collect();

        let chunky_eks: Vec<ChunkEncryptPubKey> = validator_consensus_keys
            .iter()
            .map(|k| {
                let bytes = k.to_bytes();
                bytes.as_slice().try_into().unwrap()
            })
            .collect();

        let chunky_pp = <ChunkTranscript as Transcript>::PublicParameters::new_with_commitment_base(
            chunky_config.get_total_weight(),
            aptos_dkg::pvss::chunky::DEFAULT_ELL_FOR_TESTING,
            chunky_config.get_total_num_players(),
            G2Affine::generator(),
            rng,
        );

        Self {
            epoch: cur_epoch,
            threshold_weight,
            total_weight,
            pp: chunky_pp,
            eks: chunky_eks,
            max_batch_size,
            number_of_rounds,
        }
    }

    pub fn get_threshold_config(&self) -> WeightedConfigArkworks<Fr> {
        let weights: Vec<usize> = (0..self.total_weight).map(|_| 1).collect();
        WeightedConfigArkworks::new(self.threshold_weight, weights)
            .expect("Failed to create threshold config")
    }

    pub fn get_player(&self, index: usize) -> Player {
        Player { id: index }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
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

    pub fn deserialize(&self) -> Result<ChunkTranscript> {
        bcs::from_bytes(&self.transcript_bytes)
            .context("Failed to deserialize batch encryption transcript")
    }

    pub fn serialize(&self, transcript: &ChunkTranscript) -> Result<Vec<u8>> {
        bcs::to_bytes(transcript).context("Failed to serialize batch encryption transcript")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BatchEncryptionDKGTranscriptRequest {
    pub epoch: u64,
}

impl BatchEncryptionDKGTranscriptRequest {
    pub fn new(epoch: u64) -> Self {
        Self { epoch }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchEncryptionDKGPublicParams {
    pub config: BatchEncryptionDKGConfig,
    pub verifier: Arc<ValidatorVerifier>,
}

impl BatchEncryptionDKGPublicParams {
    pub fn new(config: BatchEncryptionDKGConfig, verifier: Arc<ValidatorVerifier>) -> Self {
        Self { config, verifier }
    }

    pub fn get_transcript_request(&self) -> BatchEncryptionDKGTranscriptRequest {
        BatchEncryptionDKGTranscriptRequest::new(self.config.epoch)
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

pub struct BatchEncryptionDKG {}

impl BatchEncryptionDKG {
    pub fn generate_transcript<R: CryptoRng + RngCore>(
        rng: &mut R,
        pub_params: &BatchEncryptionDKGPublicParams,
        input_secret: &ChunkInputSecret,
        my_index: usize,
        sk: &ChunkSigningSecretKey,
        pk: &ChunkSigningPubKey,
    ) -> ChunkTranscript {
        let my_player = pub_params.config.get_player(my_index);
        let my_addr = pub_params
            .config
            .epoch
            .to_le_bytes()
            .iter()
            .chain(&my_index.to_le_bytes())
            .copied()
            .collect::<Vec<u8>>();
        let aux = my_addr;

        ChunkTranscript::deal(
            &pub_params.config.get_threshold_config(),
            &pub_params.config.pp,
            sk,
            pk,
            &pub_params.config.eks,
            input_secret,
            &aux,
            &my_player,
            rng,
        )
    }

    pub fn verify_transcript(
        transcript: &ChunkTranscript,
        pub_params: &BatchEncryptionDKGPublicParams,
        dealer_address: AccountAddress,
    ) -> Result<()> {
        let tc = pub_params.config.get_threshold_config();
        let all_eks = pub_params.config.eks.clone();
        let dealer_index = pub_params
            .verifier
            .address_to_validator_index()
            .get(&dealer_address)
            .copied()
            .ok_or_else(|| anyhow!("Dealer not found in validator set"))?;

        let spk = pub_params
            .verifier
            .get_public_key(&dealer_address)
            .ok_or_else(|| anyhow!("Dealer public key not found"))?;

        let aux = pub_params
            .config
            .epoch
            .to_le_bytes()
            .iter()
            .chain(&dealer_index.to_le_bytes())
            .copied()
            .collect::<Vec<u8>>();

        transcript
            .verify(&tc, &pub_params.config.pp, &[spk], &all_eks, &[aux])
            .context("Batch encryption transcript verification failed")
    }

    pub fn aggregate_transcripts(
        accumulator: &mut <ChunkTranscript as HasAggregatableSubtranscript>::Subtranscript,
        element: &ChunkTranscript,
        config: &BatchEncryptionDKGConfig,
    ) -> Result<()> {
        accumulator
            .aggregate_with(&config.get_threshold_config(), &element.get_subtranscript())
            .context("Failed to aggregate batch encryption transcripts")
    }

    pub fn decrypt_own_share(
        transcript: &ChunkTranscript,
        pub_params: &BatchEncryptionDKGPublicParams,
        player_index: usize,
        dk: &ChunkDecryptPrivKey,
    ) -> Result<MasterSecretKeyShare> {
        let player = pub_params.config.get_player(player_index);
        let tc = pub_params.config.get_threshold_config();

        let shamir_share_evals = transcript
            .get_subtranscript()
            .decrypt_own_share(&tc, &player, dk, &pub_params.config.pp)
            .0
            .into_iter()
            .map(|s| s.into_fr())
            .collect();

        let mpk_g2 = transcript.get_dealt_public_key().as_g2();

        Ok(MasterSecretKeyShare {
            mpk_g2,
            weighted_player: player,
            shamir_share_evals,
        })
    }

    pub fn extract_encryption_key(
        transcript: &ChunkTranscript,
        digest_key: &DigestKey,
    ) -> Result<EncryptionKey> {
        let mpk_g2 = transcript.get_dealt_public_key().as_g2();
        Ok(
            <FPTXWeighted as BatchThresholdEncryption>::EncryptionKey::new(
                mpk_g2,
                digest_key.tau_g2,
            ),
        )
    }

    pub fn extract_verification_keys(
        transcript: &ChunkTranscript,
        config: &BatchEncryptionDKGConfig,
    ) -> Result<Vec<VerificationKey>> {
        let tc = config.get_threshold_config();
        let mpk_g2 = transcript.get_dealt_public_key().as_g2();

        let num_players = tc.get_total_num_players();

        let vks: Vec<VerificationKey> = (0..num_players)
            .map(|i| {
                let player = config.get_player(i);
                let vks_g2: Vec<_> = transcript
                    .get_subtranscript()
                    .get_public_key_share(&tc, &player)
                    .into_iter()
                    .map(|vk| vk.as_g2())
                    .collect();

                VerificationKey {
                    mpk_g2,
                    weighted_player: player,
                    vks_g2,
                }
            })
            .collect();

        Ok(vks)
    }
}
