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
    type DealtSecretKey = pvss::dealt_secret_key::scalar::DealtSecretKey;
    type DealtSecretKeyShare = Vec<pvss::dealt_secret_key_share::scalar::DealtSecretKeyShare>;
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
    ) -> anyhow::Result<(Self::DealtSecretKeyShare, Self::DealtPubKeyShare)> {
        let weight = sc.get_player_weight(player);
        let mut weighted_dsk_share = Vec::with_capacity(weight);
        let mut weighted_dpk_share = Vec::with_capacity(weight);
        for i in 0..weight {
            let virtual_player = sc.get_virtual_player(player, i);
            let (dsk_share, dpk_share) =
                self.inner
                    .decrypt_own_share(sc.get_threshold_config(), &virtual_player, dk, pp)?;
            weighted_dsk_share.push(dsk_share);
            weighted_dpk_share.push(dpk_share);
        }
        Ok((weighted_dsk_share, weighted_dpk_share))
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
    use crate::pvss::{
        test_utils::{get_weighted_configs_for_testing, setup_dealing, NoAux},
        traits::{SecretSharingConfig, Transcript as TranscriptTrait},
        Player,
    };
    use aptos_crypto::ValidCryptoMaterial;
    use rand::thread_rng;

    #[test]
    fn test_weighted_transcript_compiles() {}

    /// Test that weighted encryption key expansion produces correct number of keys
    #[test]
    fn test_weighted_encryption_key_expansion() {
        let mut rng = thread_rng();

        for wc in get_weighted_configs_for_testing() {
            let d = setup_dealing::<WeightedTranscript, _>(&wc, &mut rng);

            let expanded = WeightedTranscript::to_weighted_encryption_keys(&wc, &d.eks);

            // Total expanded keys should equal total weight
            assert_eq!(
                expanded.len(),
                wc.get_total_weight(),
                "Expanded keys should equal total weight for config {}",
                wc
            );

            // Verify each player's keys are duplicated correctly
            let mut idx = 0;
            for i in 0..wc.get_total_num_players() {
                let player = wc.get_player(i);
                let weight = wc.get_player_weight(&player);
                for j in 0..weight {
                    assert_eq!(
                        expanded[idx], d.eks[i],
                        "Key at index {} should match player {}'s key (copy {})",
                        idx, i, j
                    );
                    idx += 1;
                }
            }
        }
    }

    /// Test weighted deal -> decrypt roundtrip
    #[test]
    fn test_weighted_deal_decrypt_roundtrip() {
        let mut rng = thread_rng();

        // Test with representative weighted configs
        let configs = vec![
            WeightedConfig::new(1, vec![1]).unwrap(),
            WeightedConfig::new(1, vec![1, 1]).unwrap(),
            WeightedConfig::new(2, vec![1, 1, 1]).unwrap(),
            WeightedConfig::new(3, vec![2, 1, 2]).unwrap(),
            WeightedConfig::new(3, vec![2, 3, 2]).unwrap(),
        ];

        for wc in configs {
            println!("Testing weighted deal/decrypt roundtrip for {}", wc);
            let d = setup_dealing::<WeightedTranscript, _>(&wc, &mut rng);

            let trx = WeightedTranscript::deal(
                &wc,
                &d.pp,
                &d.ssks[0],
                &d.eks,
                &d.s,
                &NoAux,
                &wc.get_player(0),
                &mut rng,
            );

            // Each player decrypts their shares
            for i in 0..wc.get_total_num_players() {
                let player = Player { id: i };
                let weight = wc.get_player_weight(&player);

                let (sk_shares, pk_shares) = trx
                    .decrypt_own_share(&wc, &player, &d.dks[i], &d.pp)
                    .expect("decrypt_own_share should not fail for valid transcript");

                // Verify correct number of shares for this player's weight
                assert_eq!(
                    sk_shares.len(),
                    weight,
                    "Player {} should have {} secret key shares",
                    i,
                    weight
                );
                assert_eq!(
                    pk_shares.len(),
                    weight,
                    "Player {} should have {} public key shares",
                    i,
                    weight
                );

                // Verify public key shares match transcript
                let expected_pk_shares = trx.get_public_key_share(&wc, &player);
                assert_eq!(
                    pk_shares, expected_pk_shares,
                    "Public key shares mismatch for player {} in config {}",
                    i, wc
                );
            }
        }
    }

    /// Test weighted aggregation
    #[test]
    fn test_weighted_aggregation() {
        let mut rng = thread_rng();
        let wc = WeightedConfig::new(3, vec![2, 1, 2]).unwrap();

        let d = setup_dealing::<WeightedTranscript, _>(&wc, &mut rng);

        // Create transcripts from multiple dealers
        let mut trx1 = WeightedTranscript::deal(
            &wc,
            &d.pp,
            &d.ssks[0],
            &d.eks,
            &d.iss[0],
            &NoAux,
            &wc.get_player(0),
            &mut rng,
        );

        let trx2 = WeightedTranscript::deal(
            &wc,
            &d.pp,
            &d.ssks[1],
            &d.eks,
            &d.iss[1],
            &NoAux,
            &wc.get_player(1),
            &mut rng,
        );

        let trx3 = WeightedTranscript::deal(
            &wc,
            &d.pp,
            &d.ssks[2],
            &d.eks,
            &d.iss[2],
            &NoAux,
            &wc.get_player(2),
            &mut rng,
        );

        // Aggregate
        trx1.aggregate_with(&wc, &trx2);
        trx1.aggregate_with(&wc, &trx3);

        // Verify dealers are tracked
        let dealers = trx1.get_dealers();
        assert_eq!(dealers.len(), 3);

        // Verify dealt public key matches combined secrets
        let aggregated_dpk = trx1.get_dealt_public_key();
        assert_eq!(
            aggregated_dpk, d.dpk,
            "Aggregated public key should match sum of input secrets"
        );

        // Each player can still decrypt their shares
        for i in 0..wc.get_total_num_players() {
            let player = Player { id: i };
            let (sk_shares, pk_shares) = trx1
                .decrypt_own_share(&wc, &player, &d.dks[i], &d.pp)
                .expect("decrypt_own_share should not fail for valid transcript");

            let expected_pk_shares = trx1.get_public_key_share(&wc, &player);
            assert_eq!(
                pk_shares, expected_pk_shares,
                "Public key shares should match after aggregation for player {}",
                i
            );

            // Verify we got the right number of shares
            let weight = wc.get_player_weight(&player);
            assert_eq!(sk_shares.len(), weight);
        }
    }

    /// Test weighted serialization roundtrip
    #[test]
    fn test_weighted_serialization_roundtrip() {
        let mut rng = thread_rng();
        let wc = WeightedConfig::new(2, vec![1, 2, 1]).unwrap();

        let d = setup_dealing::<WeightedTranscript, _>(&wc, &mut rng);

        let trx = WeightedTranscript::deal(
            &wc,
            &d.pp,
            &d.ssks[0],
            &d.eks,
            &d.s,
            &NoAux,
            &wc.get_player(0),
            &mut rng,
        );

        // Serialize
        let bytes = trx.to_bytes();

        // Deserialize
        let trx_restored =
            WeightedTranscript::try_from(bytes.as_slice()).expect("Deserialization should succeed");

        // Verify equality
        assert_eq!(
            trx, trx_restored,
            "WeightedTranscript should survive serialization roundtrip"
        );
    }

    /// Test dealt public key consistency for weighted config
    #[test]
    fn test_weighted_dealt_public_key_consistency() {
        let mut rng = thread_rng();
        let wc = WeightedConfig::new(3, vec![2, 3, 2]).unwrap();

        let d = setup_dealing::<WeightedTranscript, _>(&wc, &mut rng);

        let trx = WeightedTranscript::deal(
            &wc,
            &d.pp,
            &d.ssks[0],
            &d.eks,
            &d.s,
            &NoAux,
            &wc.get_player(0),
            &mut rng,
        );

        let dpk = trx.get_dealt_public_key();

        // Verify it matches the expected public key from the input secret
        assert_eq!(
            dpk, d.dpk,
            "Dealt public key should match expected from input secret"
        );
    }

    /// Test with realistic validator-like weights
    #[test]
    fn test_weighted_realistic_validator_weights() {
        let mut rng = thread_rng();

        // Simulate a small validator set with varying stake
        let weights = vec![10, 5, 15, 8, 12]; // 5 validators, total weight 50
        let threshold = 34; // ~2/3 + 1
        let wc = WeightedConfig::new(threshold, weights).unwrap();

        let d = setup_dealing::<WeightedTranscript, _>(&wc, &mut rng);

        // Deal from all validators
        let mut aggregated = WeightedTranscript::deal(
            &wc,
            &d.pp,
            &d.ssks[0],
            &d.eks,
            &d.iss[0],
            &NoAux,
            &wc.get_player(0),
            &mut rng,
        );

        for i in 1..wc.get_total_num_players() {
            let trx = WeightedTranscript::deal(
                &wc,
                &d.pp,
                &d.ssks[i],
                &d.eks,
                &d.iss[i],
                &NoAux,
                &wc.get_player(i),
                &mut rng,
            );
            aggregated.aggregate_with(&wc, &trx);
        }

        // Verify aggregated public key
        assert_eq!(
            aggregated.get_dealt_public_key(),
            d.dpk,
            "Aggregated DPK should match expected"
        );

        // All validators can decrypt their proportional shares
        for i in 0..wc.get_total_num_players() {
            let player = Player { id: i };
            let (sk_shares, pk_shares) = aggregated
                .decrypt_own_share(&wc, &player, &d.dks[i], &d.pp)
                .expect("decrypt_own_share should not fail for valid transcript");

            let expected_weight = wc.get_player_weight(&player);
            assert_eq!(
                sk_shares.len(),
                expected_weight,
                "Validator {} should have {} shares",
                i,
                expected_weight
            );
            assert_eq!(pk_shares.len(), expected_weight);
        }
    }
}
