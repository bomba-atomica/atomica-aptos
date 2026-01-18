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

    #[test]
    fn test_transcript_struct_compiles() {}
}
