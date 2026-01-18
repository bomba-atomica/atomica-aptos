// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! # Weighted Scalar ElGamal PVSS
//!
//! This module implements weighted secret sharing for the Scalar ElGamal PVSS scheme.
//! It wraps the core unweighted `Transcript` to support validator stake-proportional weights.

use super::transcript::Transcript;
use crate::pvss::{
    self, das, encryption_dlog,
    traits::{SecretSharingConfig, Transcript as TranscriptTrait},
    Player, WeightedConfig,
};
use anyhow::Result;
use aptos_crypto::{bls12381, CryptoMaterialError, ValidCryptoMaterial};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use serde::{Deserialize, Serialize};

pub const WEIGHTED_SCHEME_NAME: &str = "weighted_scalar_elgamal_pvss";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, BCSCryptoHash, CryptoHasher)]
pub struct WeightedTranscript {
    inner: Transcript,
}

impl ValidCryptoMaterial for WeightedTranscript {
    const AIP_80_PREFIX: &'static str = "";

    fn to_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(&self).expect("unexpected error during weighted transcript serialization")
    }
}

impl TryFrom<&[u8]> for WeightedTranscript {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        bcs::from_bytes::<WeightedTranscript>(bytes)
            .map_err(|_| CryptoMaterialError::DeserializationError)
    }
}

impl WeightedTranscript {
    fn to_weighted_encryption_keys(
        sc: &WeightedConfig,
        eks: &Vec<encryption_dlog::g1::EncryptPubKey>,
    ) -> Vec<encryption_dlog::g1::EncryptPubKey> {
        let mut duplicated_eks = Vec::with_capacity(sc.get_total_weight());
        for (player_id, ek) in eks.iter().enumerate() {
            let player = sc.get_player(player_id);
            let num_shares = sc.get_player_weight(&player);
            for _ in 0..num_shares {
                duplicated_eks.push(ek.clone());
            }
        }
        duplicated_eks
    }
}

impl TranscriptTrait for WeightedTranscript {
    type DealtPubKey = pvss::dealt_pub_key::g2::DealtPubKey;
    type DealtPubKeyShare = Vec<pvss::dealt_pub_key_share::g2::DealtPubKeyShare>;
    type DealtSecretKey = pvss::dealt_secret_key::g1::DealtSecretKey;
    type DealtSecretKeyShare = Vec<pvss::dealt_secret_key_share::g1::DealtSecretKeyShare>;
    type DecryptPrivKey = encryption_dlog::g1::DecryptPrivKey;
    type EncryptPubKey = encryption_dlog::g1::EncryptPubKey;
    type InputSecret = pvss::input_secret::InputSecret;
    type PublicParameters = das::PublicParameters;
    type SecretSharingConfig = WeightedConfig;
    type SigningPubKey = bls12381::PublicKey;
    type SigningSecretKey = bls12381::PrivateKey;

    fn dst() -> Vec<u8> {
        let mut result = b"WEIGHTED_".to_vec();
        result.extend(super::SCALAR_ELGAMAL_DST);
        result
    }

    fn scheme_name() -> String {
        WEIGHTED_SCHEME_NAME.to_string()
    }

    fn deal<A: Serialize + Clone, R: rand_core::RngCore + rand_core::CryptoRng>(
        sc: &Self::SecretSharingConfig,
        pp: &Self::PublicParameters,
        ssk: &Self::SigningSecretKey,
        eks: &Vec<Self::EncryptPubKey>,
        s: &Self::InputSecret,
        aux: &A,
        dealer: &Player,
        rng: &mut R,
    ) -> Self {
        let duplicated_eks = Self::to_weighted_encryption_keys(sc, eks);
        let inner = Transcript::deal(
            sc.get_threshold_config(),
            pp,
            ssk,
            &duplicated_eks,
            s,
            aux,
            dealer,
            rng,
        );
        WeightedTranscript { inner }
    }

    fn verify<A: Serialize + Clone>(
        &self,
        sc: &Self::SecretSharingConfig,
        pp: &Self::PublicParameters,
        spks: &Vec<Self::SigningPubKey>,
        eks: &Vec<Self::EncryptPubKey>,
        auxs: &Vec<A>,
    ) -> Result<()> {
        let duplicated_eks = Self::to_weighted_encryption_keys(sc, eks);
        self.inner
            .verify(sc.get_threshold_config(), pp, spks, &duplicated_eks, auxs)
    }

    fn get_dealers(&self) -> Vec<Player> {
        self.inner.get_dealers()
    }

    fn aggregate_with(&mut self, sc: &Self::SecretSharingConfig, other: &Self) {
        self.inner
            .aggregate_with(sc.get_threshold_config(), &other.inner);
    }

    fn get_public_key_share(
        &self,
        sc: &Self::SecretSharingConfig,
        player: &Player,
    ) -> Self::DealtPubKeyShare {
        let weight = sc.get_player_weight(player);
        let mut dpk_share = Vec::with_capacity(weight);
        for i in 0..weight {
            let virtual_player = sc.get_virtual_player(player, i);
            dpk_share.push(
                self.inner
                    .get_public_key_share(sc.get_threshold_config(), &virtual_player),
            );
        }
        dpk_share
    }

    fn get_dealt_public_key(&self) -> Self::DealtPubKey {
        self.inner.get_dealt_public_key()
    }

    fn decrypt_own_share(
        &self,
        sc: &Self::SecretSharingConfig,
        player: &Player,
        dk: &Self::DecryptPrivKey,
        pp: &Self::PublicParameters,
    ) -> (Self::DealtSecretKeyShare, Self::DealtPubKeyShare) {
        let weight = sc.get_player_weight(player);
        let mut weighted_dsk_share = Vec::with_capacity(weight);
        let mut weighted_dpk_share = Vec::with_capacity(weight);
        for i in 0..weight {
            let virtual_player = sc.get_virtual_player(player, i);
            let (dsk_share, dpk_share) =
                self.inner
                    .decrypt_own_share(sc.get_threshold_config(), &virtual_player, dk, pp);
            weighted_dsk_share.push(dsk_share);
            weighted_dpk_share.push(dpk_share);
        }
        (weighted_dsk_share, weighted_dpk_share)
    }

    fn generate<R>(_sc: &Self::SecretSharingConfig, _rng: &mut R) -> Self
    where
        R: rand_core::RngCore + rand_core::CryptoRng,
    {
        todo!("Implement generate() for testing")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weighted_transcript_compiles() {}
}
