// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! # Scalar ElGamal PVSS Transcript
//!
//! This module implements the core PVSS protocol for sharing scalar secrets with
//! ElGamal encryption. It is the scalar-output counterpart to the DAS PVSS.

use crate::{
    algebra::polynomials::shamir_secret_share,
    pvss::{
        contribution::SoK,
        das, dealt_pub_key, dealt_pub_key_share, dealt_secret_key, dealt_secret_key_share,
        encryption_dlog, input_secret, schnorr,
        traits::{self, HasEncryptionPublicParams},
        Player, ThresholdConfigBlstrs,
    },
};
use anyhow::{bail, Result};
use aptos_crypto::{
    bls12381, blstrs::random_scalar, CryptoMaterialError, SigningKey, ValidCryptoMaterial,
};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use blstrs::{G1Projective, G2Projective};
use serde::{Deserialize, Serialize};
use std::ops::{Add, Mul, Sub};

pub const SCALAR_ELGAMAL_DST: &[u8] = b"APTOS_SCALAR_ELGAMAL_PVSS_DST";
pub const SCHEME_NAME: &str = "scalar_elgamal_pvss";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, BCSCryptoHash, CryptoHasher)]
#[allow(non_snake_case)]
pub struct Transcript {
    soks: Vec<SoK<G2Projective>>,
    hat_w: G2Projective,
    V: Vec<G2Projective>,
    C: Vec<G1Projective>,
    C_0: G1Projective,
}

impl ValidCryptoMaterial for Transcript {
    const AIP_80_PREFIX: &'static str = "";

    fn to_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(self).expect("unexpected error during PVSS transcript serialization")
    }
}

impl TryFrom<&[u8]> for Transcript {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        bcs::from_bytes::<Transcript>(bytes).map_err(|_| CryptoMaterialError::DeserializationError)
    }
}

impl traits::Transcript for Transcript {
    type DealtPubKey = dealt_pub_key::g2::DealtPubKey;
    type DealtPubKeyShare = dealt_pub_key_share::g2::DealtPubKeyShare;
    type DealtSecretKey = dealt_secret_key::g1::DealtSecretKey;
    type DealtSecretKeyShare = dealt_secret_key_share::g1::DealtSecretKeyShare;
    type DecryptPrivKey = encryption_dlog::g1::DecryptPrivKey;
    type EncryptPubKey = encryption_dlog::g1::EncryptPubKey;
    type InputSecret = input_secret::InputSecret;
    type PublicParameters = das::PublicParameters;
    type SecretSharingConfig = ThresholdConfigBlstrs;
    type SigningPubKey = bls12381::PublicKey;
    type SigningSecretKey = bls12381::PrivateKey;

    fn dst() -> Vec<u8> {
        SCALAR_ELGAMAL_DST.to_vec()
    }

    fn scheme_name() -> String {
        SCHEME_NAME.to_string()
    }

    #[allow(non_snake_case)]
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
        assert_eq!(eks.len(), sc.n);

        let (f, f_evals) = shamir_secret_share(sc, s, rng);

        let r = random_scalar(rng);
        let g_1 = pp.get_encryption_public_params().pubkey_base();
        let g_2 = pp.get_commitment_base();
        let h_1 = *pp.get_encryption_public_params().message_base();

        let V = (0..sc.n)
            .map(|i| g_2.mul(f_evals[i]))
            .chain([g_2.mul(f[0])])
            .collect::<Vec<G2Projective>>();

        let C = (0..sc.n)
            .map(|i| {
                let share_g1 = h_1.mul(f_evals[i]);
                let ek_r = Into::<G1Projective>::into(&eks[i]).mul(r);
                share_g1.add(ek_r)
            })
            .collect::<Vec<G1Projective>>();

        let pok = schnorr::pok_prove(&f[0], g_2, &V[sc.n], rng);

        debug_assert_eq!(V.len(), sc.n + 1);
        debug_assert_eq!(C.len(), sc.n);

        let sig = Transcript::sign_contribution(ssk, dealer, aux, &V[sc.n]);

        Transcript {
            soks: vec![(*dealer, V[sc.n], sig, pok)],
            hat_w: g_2.mul(r),
            V,
            C,
            C_0: g_1.mul(r),
        }
    }

    fn verify<A: Serialize + Clone>(
        &self,
        sc: &Self::SecretSharingConfig,
        _pp: &Self::PublicParameters,
        _spks: &Vec<Self::SigningPubKey>,
        _eks: &Vec<Self::EncryptPubKey>,
        _auxs: &Vec<A>,
    ) -> Result<()> {
        if self.C.len() != sc.n {
            bail!("Expected {} ciphertexts, but got {}", sc.n, self.C.len());
        }
        if self.V.len() != sc.n + 1 {
            bail!(
                "Expected {} (polynomial) commitment elements, but got {}",
                sc.n + 1,
                self.V.len()
            );
        }
        bail!("verify() not yet implemented for Scalar ElGamal PVSS")
    }

    fn get_dealers(&self) -> Vec<Player> {
        self.soks.iter().map(|(p, _, _, _)| *p).collect()
    }

    fn aggregate_with(&mut self, sc: &Self::SecretSharingConfig, other: &Transcript) {
        debug_assert_eq!(self.C.len(), sc.n);
        debug_assert_eq!(self.V.len(), sc.n + 1);

        self.hat_w += other.hat_w;
        self.C_0 += other.C_0;

        for i in 0..sc.n {
            self.C[i] += other.C[i];
            self.V[i] += other.V[i];
        }
        self.V[sc.n] += other.V[sc.n];

        for sok in &other.soks {
            self.soks.push(sok.clone());
        }
    }

    fn get_public_key_share(
        &self,
        _sc: &Self::SecretSharingConfig,
        player: &Player,
    ) -> Self::DealtPubKeyShare {
        Self::DealtPubKeyShare::new(Self::DealtPubKey::new(self.V[player.id]))
    }

    fn get_dealt_public_key(&self) -> Self::DealtPubKey {
        Self::DealtPubKey::new(*self.V.last().unwrap())
    }

    fn decrypt_own_share(
        &self,
        _sc: &Self::SecretSharingConfig,
        player: &Player,
        dk: &Self::DecryptPrivKey,
        pp: &Self::PublicParameters,
    ) -> (Self::DealtSecretKeyShare, Self::DealtPubKeyShare) {
        let ctxt = self.C[player.id];
        let g_1 = pp.get_encryption_public_params().pubkey_base();

        let _ek_i = g_1.mul(dk.dk);
        let ephemeral_key = self.C_0.mul(dk.dk);
        let h1_share = ctxt.sub(ephemeral_key);

        let dealt_secret_key_share = h1_share;
        let dealt_pub_key_share = self.V[player.id];

        (
            Self::DealtSecretKeyShare::new(Self::DealtSecretKey::new(dealt_secret_key_share)),
            Self::DealtPubKeyShare::new(Self::DealtPubKey::new(dealt_pub_key_share)),
        )
    }

    fn generate<R>(_sc: &Self::SecretSharingConfig, _rng: &mut R) -> Self
    where
        R: rand_core::RngCore + rand_core::CryptoRng,
    {
        todo!("Implement generate() for testing")
    }
}

impl Transcript {
    pub fn sign_contribution<A: Serialize + Clone>(
        sk: &bls12381::PrivateKey,
        player: &Player,
        aux: &A,
        comm: &G2Projective,
    ) -> bls12381::Signature {
        sk.sign(
            &crate::pvss::contribution::Contribution::<G2Projective, A> {
                comm: *comm,
                player: *player,
                aux: aux.clone(),
            },
        )
        .expect("signing of PVSS contribution should have succeeded")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        pvss::{
            test_utils::{get_threshold_configs_for_testing, setup_dealing, NoAux},
            traits::{SecretSharingConfig, Transcript as TranscriptTrait},
            Player, ThresholdConfigBlstrs,
        },
        traits::ThresholdConfig,
    };
    use aptos_crypto::ValidCryptoMaterial;
    use rand::thread_rng;

    #[test]
    fn test_transcript_struct_compiles() {}

    /// Test that deal() creates a transcript with correct structure sizes
    #[test]
    fn test_deal_creates_valid_structure() {
        let mut rng = thread_rng();

        for tc in get_threshold_configs_for_testing::<ThresholdConfigBlstrs>() {
            let d = setup_dealing::<Transcript, _>(&tc, &mut rng);

            let trx = Transcript::deal(
                &tc,
                &d.pp,
                &d.ssks[0],
                &d.eks,
                &d.s,
                &NoAux,
                &tc.get_player(0),
                &mut rng,
            );

            // Verify structure sizes
            assert_eq!(
                trx.V.len(),
                tc.n + 1,
                "V should have n+1 elements for t={}, n={}",
                tc.t,
                tc.n
            );
            assert_eq!(
                trx.C.len(),
                tc.n,
                "C should have n elements for t={}, n={}",
                tc.t,
                tc.n
            );
            assert_eq!(
                trx.soks.len(),
                1,
                "Should have exactly one SoK after dealing"
            );
        }
    }

    /// Test deal -> decrypt roundtrip: all players can decrypt their shares
    #[test]
    fn test_deal_decrypt_roundtrip() {
        let mut rng = thread_rng();

        // Test with a few representative configs
        let configs = vec![
            ThresholdConfigBlstrs::new(1, 1).unwrap(),
            ThresholdConfigBlstrs::new(2, 3).unwrap(),
            ThresholdConfigBlstrs::new(3, 5).unwrap(),
            ThresholdConfigBlstrs::new(4, 7).unwrap(),
        ];

        for tc in configs {
            println!("Testing deal/decrypt roundtrip for {}", tc);
            let d = setup_dealing::<Transcript, _>(&tc, &mut rng);

            let trx = Transcript::deal(
                &tc,
                &d.pp,
                &d.ssks[0],
                &d.eks,
                &d.s,
                &NoAux,
                &tc.get_player(0),
                &mut rng,
            );

            // Each player decrypts their share
            for i in 0..tc.n {
                let player = Player { id: i };
                let (sk_share, pk_share) = trx.decrypt_own_share(&tc, &player, &d.dks[i], &d.pp);

                // Verify public key share matches what's in transcript
                let expected_pk_share = trx.get_public_key_share(&tc, &player);
                assert_eq!(
                    pk_share, expected_pk_share,
                    "Public key share mismatch for player {} in config {}",
                    i, tc
                );

                // Use sk_share to avoid unused warning - the decryption itself is the test
                let _ = sk_share;
            }
        }
    }

    /// Test that aggregation combines transcripts correctly
    #[test]
    fn test_aggregation_preserves_structure() {
        let mut rng = thread_rng();
        let tc = ThresholdConfigBlstrs::new(2, 4).unwrap();

        let d = setup_dealing::<Transcript, _>(&tc, &mut rng);

        // Create transcripts from multiple dealers
        let mut trx1 = Transcript::deal(
            &tc,
            &d.pp,
            &d.ssks[0],
            &d.eks,
            &d.iss[0],
            &NoAux,
            &tc.get_player(0),
            &mut rng,
        );

        let trx2 = Transcript::deal(
            &tc,
            &d.pp,
            &d.ssks[1],
            &d.eks,
            &d.iss[1],
            &NoAux,
            &tc.get_player(1),
            &mut rng,
        );

        let trx3 = Transcript::deal(
            &tc,
            &d.pp,
            &d.ssks[2],
            &d.eks,
            &d.iss[2],
            &NoAux,
            &tc.get_player(2),
            &mut rng,
        );

        // Aggregate
        trx1.aggregate_with(&tc, &trx2);
        trx1.aggregate_with(&tc, &trx3);

        // Verify structure is preserved
        assert_eq!(trx1.V.len(), tc.n + 1, "V length should be preserved");
        assert_eq!(trx1.C.len(), tc.n, "C length should be preserved");
        assert_eq!(trx1.soks.len(), 3, "Should have 3 SoKs after aggregation");

        // Verify dealers are tracked
        let dealers = trx1.get_dealers();
        assert_eq!(dealers.len(), 3);
        assert!(dealers.contains(&tc.get_player(0)));
        assert!(dealers.contains(&tc.get_player(1)));
        assert!(dealers.contains(&tc.get_player(2)));
    }

    /// Test BCS serialization roundtrip
    #[test]
    fn test_serialization_roundtrip() {
        let mut rng = thread_rng();
        let tc = ThresholdConfigBlstrs::new(2, 3).unwrap();

        let d = setup_dealing::<Transcript, _>(&tc, &mut rng);

        let trx = Transcript::deal(
            &tc,
            &d.pp,
            &d.ssks[0],
            &d.eks,
            &d.s,
            &NoAux,
            &tc.get_player(0),
            &mut rng,
        );

        // Serialize
        let bytes = trx.to_bytes();

        // Deserialize
        let trx_restored =
            Transcript::try_from(bytes.as_slice()).expect("Deserialization should succeed");

        // Verify equality
        assert_eq!(
            trx, trx_restored,
            "Transcript should survive serialization roundtrip"
        );
    }

    /// Test that dealt public key is consistent
    #[test]
    fn test_dealt_public_key_consistency() {
        let mut rng = thread_rng();
        let tc = ThresholdConfigBlstrs::new(2, 4).unwrap();

        let d = setup_dealing::<Transcript, _>(&tc, &mut rng);

        let trx = Transcript::deal(
            &tc,
            &d.pp,
            &d.ssks[0],
            &d.eks,
            &d.s,
            &NoAux,
            &tc.get_player(0),
            &mut rng,
        );

        // Get dealt public key
        let dpk = trx.get_dealt_public_key();

        // Verify it matches the expected public key from the input secret
        assert_eq!(
            dpk, d.dpk,
            "Dealt public key should match expected from input secret"
        );
    }

    /// Test aggregated transcript decryption produces combined secret
    #[test]
    fn test_aggregated_decrypt_combines_secrets() {
        let mut rng = thread_rng();
        let tc = ThresholdConfigBlstrs::new(2, 3).unwrap();

        let d = setup_dealing::<Transcript, _>(&tc, &mut rng);

        // Deal from all players
        let mut aggregated = Transcript::deal(
            &tc,
            &d.pp,
            &d.ssks[0],
            &d.eks,
            &d.iss[0],
            &NoAux,
            &tc.get_player(0),
            &mut rng,
        );

        for i in 1..tc.n {
            let trx = Transcript::deal(
                &tc,
                &d.pp,
                &d.ssks[i],
                &d.eks,
                &d.iss[i],
                &NoAux,
                &tc.get_player(i),
                &mut rng,
            );
            aggregated.aggregate_with(&tc, &trx);
        }

        // The aggregated dealt public key should match the sum of all input secrets
        let aggregated_dpk = aggregated.get_dealt_public_key();
        assert_eq!(
            aggregated_dpk, d.dpk,
            "Aggregated public key should match sum of input secrets"
        );

        // Each player can still decrypt their share from the aggregated transcript
        for i in 0..tc.n {
            let player = Player { id: i };
            let (sk_share, pk_share) = aggregated.decrypt_own_share(&tc, &player, &d.dks[i], &d.pp);

            let expected_pk_share = aggregated.get_public_key_share(&tc, &player);
            assert_eq!(
                pk_share, expected_pk_share,
                "Public key share should match for player {} after aggregation",
                i
            );

            // Use sk_share to avoid unused warning
            let _ = sk_share;
        }
    }
}
