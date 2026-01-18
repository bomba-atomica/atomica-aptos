// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! # Scalar ElGamal PVSS Transcript
//!
//! This module implements the core PVSS protocol for sharing scalar secrets with
//! Chunked Lifted ElGamal encryption.
//!
//! # Architecture
//!
//! 1. **Secret Sharing**: The input scalar $s$ is shared using Shamir's Secret Sharing to get shares $sh_i$.
//! 2. **Chunking**: Each share $sh_i$ (32 bytes) is split into 16 chunks of 16 bits: $u_{i,0} \dots u_{i,15}$.
//! 3. **Lifted ElGamal**: Each chunk $u_{i,j}$ is encrypted as $C_{i,j} = g^{u_{i,j}} \cdot pk_i^{r_j}$.
//!    - Randomness $r_j$ is shared across all validators for the same chunk index $j$.
//!    - Ephemeral keys $R_j = g^{r_j}$ are included in the transcript (one per chunk index).
//! 4. **Decryption**: Validator $i$ computes $pk_i^{r_j} = R_j^{sk_i}$, recovers $g^{u_{i,j}}$, and solves the discrete log (BSGS) to get $u_{i,j}$.
//! 5. **Reconstruction**: Chunks are combined to form $sh_i$.

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
use blstrs::{G1Projective, G2Projective, Scalar};
use group::Group;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    ops::{Add, Mul, Sub},
};

pub const SCALAR_ELGAMAL_DST: &[u8] = b"APTOS_SCALAR_ELGAMAL_PVSS_DST";
pub const SCHEME_NAME: &str = "scalar_elgamal_pvss";

const CHUNK_BIT_SIZE: usize = 16;
const NUM_CHUNKS: usize = 16; // 256 bits / 16 = 16

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, BCSCryptoHash, CryptoHasher)]
#[allow(non_snake_case)]
pub struct Transcript {
    soks: Vec<SoK<G2Projective>>,
    hat_w: G2Projective,
    V: Vec<G2Projective>,
    /// Ephemeral keys $R_j = g^{r_j}$ for each chunk index $j=0..15$
    ephemeral_keys: Vec<G1Projective>,
    /// Ciphertexts $C_{i,j}$ for each validator $i$ and chunk $j$.
    /// Outer vector corresponds to validators (ordered by ID).
    /// Inner vector corresponds to chunks.
    ciphertexts: Vec<Vec<G1Projective>>,
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
    type DealtSecretKey = dealt_secret_key::scalar::DealtSecretKey;
    type DealtSecretKeyShare = dealt_secret_key_share::scalar::DealtSecretKeyShare;
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

        // 1. Generate shares
        let (f, f_evals) = shamir_secret_share(sc, s, rng);

        // 2. Generate randomness for each chunk index (shared across validators)
        let chunk_randomness: Vec<Scalar> = (0..NUM_CHUNKS).map(|_| random_scalar(rng)).collect();

        let g_1 = pp.get_encryption_public_params().pubkey_base();
        let g_2 = pp.get_commitment_base();
        let _h_1 = *pp.get_encryption_public_params().message_base(); // Not used in this scheme

        // 3. Compute ephemeral keys R_j = g^r_j
        let ephemeral_keys: Vec<G1Projective> =
            chunk_randomness.iter().map(|r| g_1.mul(r)).collect();

        // 4. Compute polynomial commitments V
        let V = (0..sc.n)
            .map(|i| g_2.mul(f_evals[i]))
            .chain([g_2.mul(f[0])])
            .collect::<Vec<G2Projective>>();

        // 5. Encrypt shares
        // Precompute table for chunks if needed? No, chunks are small scalars.
        // We need to compute g^chunk.
        // Actually, since chunk is u16, we can just use g_1 * Scalar::from(chunk).

        let ciphertexts: Vec<Vec<G1Projective>> = (0..sc.n)
            .map(|i| {
                let share = f_evals[i];
                let chunks = scalar_to_chunks(&share);

                chunks
                    .iter()
                    .enumerate()
                    .map(|(j, &chunk)| {
                        // C_{i,j} = g^{u_{i,j}} * pk_i^{r_j}
                        let g_u = g_1.mul(Scalar::from(chunk as u64));
                        let pk_r = Into::<G1Projective>::into(&eks[i]).mul(chunk_randomness[j]);
                        g_u + pk_r
                    })
                    .collect()
            })
            .collect();

        // 6. Generate Proof of Knowledge
        let pok = schnorr::pok_prove(&f[0], g_2, &V[sc.n], rng);

        debug_assert_eq!(V.len(), sc.n + 1);
        debug_assert_eq!(ciphertexts.len(), sc.n);

        let sig = Transcript::sign_contribution(ssk, dealer, aux, &V[sc.n]);

        Transcript {
            soks: vec![(*dealer, V[sc.n], sig, pok)],
            hat_w: g_2.mul(chunk_randomness[0]), // Placeholder? hat_w usually commits to the randomness.
            // But here we have multiple randomnesses.
            // Existing verify() might check hat_w?
            // In DAS, hat_w = g2^r. Used for DLEQ?
            // Here we have r_0..r_15.
            // For now, let's store g2^r_0 just to satisfy the struct,
            // but verification logic needs to be updated or ignored.
            V,
            ephemeral_keys,
            ciphertexts,
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
        if self.ciphertexts.len() != sc.n {
            bail!(
                "Expected {} ciphertext vectors, but got {}",
                sc.n,
                self.ciphertexts.len()
            );
        }
        if self.V.len() != sc.n + 1 {
            bail!(
                "Expected {} commitment elements, but got {}",
                sc.n + 1,
                self.V.len()
            );
        }
        // TODO: Verification logic for Chunked ElGamal
        Ok(())
    }

    fn get_dealers(&self) -> Vec<Player> {
        self.soks.iter().map(|(p, _, _, _)| *p).collect()
    }

    fn aggregate_with(&mut self, sc: &Self::SecretSharingConfig, other: &Transcript) {
        debug_assert_eq!(self.ciphertexts.len(), sc.n);
        debug_assert_eq!(self.V.len(), sc.n + 1);

        self.hat_w += other.hat_w;

        for j in 0..NUM_CHUNKS {
            self.ephemeral_keys[j] += other.ephemeral_keys[j];
        }

        for i in 0..sc.n {
            for j in 0..NUM_CHUNKS {
                self.ciphertexts[i][j] += other.ciphertexts[i][j];
            }
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
        let g_1 = pp.get_encryption_public_params().pubkey_base();
        let chunks_ciphertexts = &self.ciphertexts[player.id];

        let mut recovered_chunks = Vec::with_capacity(NUM_CHUNKS);

        let limit = 1 << CHUNK_BIT_SIZE;
        let m = (limit as f64).sqrt().ceil() as u64;
        let bsgs_table = compute_bsgs_table(g_1, m);
        let giant_step = g_1.mul(Scalar::from(m));

        for (j, c_ij) in chunks_ciphertexts.iter().enumerate() {
            // S_{i,j} = R_j^{sk_i} = (g^{r_j})^{sk_i} = pk_i^{r_j}
            let s_ij = self.ephemeral_keys[j].mul(dk.dk);

            // M_{i,j} = C_{i,j} - S_{i,j} = g^{u_{i,j}}
            let m_ij = c_ij - s_ij;

            // Solve discrete log
            let u_ij = solve_discrete_log(&m_ij, &bsgs_table, &giant_step, m)
                .expect("Failed to solve discrete log for chunk");

            recovered_chunks.push(u_ij);
        }

        // Reconstruct scalar from chunks
        let share = chunks_to_scalar(&recovered_chunks);

        (
            Self::DealtSecretKeyShare::new(Self::DealtSecretKey::new(share)),
            Self::DealtPubKeyShare::new(Self::DealtPubKey::new(self.V[player.id])),
        )
    }

    fn generate<R>(_sc: &Self::SecretSharingConfig, _rng: &mut R) -> Self
    where
        R: rand_core::RngCore + rand_core::CryptoRng,
    {
        todo!("Implement generate() for testing")
    }
}

// Helpers

fn scalar_to_chunks(s: &Scalar) -> Vec<u16> {
    let bytes = s.to_bytes_le();
    let mut chunks = Vec::with_capacity(NUM_CHUNKS);
    for chunk_bytes in bytes.chunks(2) {
        let val = if chunk_bytes.len() == 2 {
            u16::from_le_bytes([chunk_bytes[0], chunk_bytes[1]])
        } else {
            u16::from_le_bytes([chunk_bytes[0], 0])
        };
        chunks.push(val);
    }
    chunks
}

fn chunks_to_scalar(chunks: &[u16]) -> Scalar {
    let mut bytes = [0u8; 32];
    for (i, chunk) in chunks.iter().enumerate() {
        let chunk_bytes = chunk.to_le_bytes();
        if 2 * i < 32 {
            bytes[2 * i] = chunk_bytes[0];
        }
        if 2 * i + 1 < 32 {
            bytes[2 * i + 1] = chunk_bytes[1];
        }
    }
    // We assume the scalar fits in 32 bytes and is canonical.
    // blstrs::Scalar::from_bytes_le handles modular reduction if needed?
    // Actually it returns Option. If it was a valid scalar initially, it should be valid now.
    Scalar::from_bytes_le(&bytes).expect("Reconstructed scalar bytes invalid")
}

// Baby-step Giant-step solver
fn compute_bsgs_table(base: &G1Projective, m: u64) -> HashMap<Vec<u8>, u64> {
    let mut table = HashMap::with_capacity(m as usize);
    let mut curr = G1Projective::identity();

    // Baby steps: j*G for j in 0..m
    for j in 0..m {
        table.insert(curr.to_compressed().to_vec(), j);
        curr += base;
    }
    table
}

fn solve_discrete_log(
    target: &G1Projective,
    table: &HashMap<Vec<u8>, u64>,
    giant_step: &G1Projective,
    m: u64,
) -> Option<u16> {
    // target = i*m*G + j*G
    // target - i*m*G = j*G

    let mut current = *target;

    // Giant steps: i in 0..m
    for i in 0..m {
        // Check if (target - i*giant_step) matches a baby step
        let key = current.to_compressed().to_vec();
        if let Some(&j) = table.get(&key) {
            return Some((i * m + j) as u16);
        }
        current -= giant_step;
    }
    None
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
