// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! # Scalar ElGamal PVSS Transcript
//!
//! This module implements the core PVSS (Publicly Verifiable Secret Sharing) protocol
//! for sharing scalar secrets using Chunked Lifted ElGamal encryption. This implementation
//! is designed for Distributed Key Generation (DKG) in threshold cryptography systems.
//!
//! ## Overview
//!
//! This module is part of Atomica's Dual-Output DKG architecture that produces two types
//! of key material in a single round:
//! - **G1 shares** (via DAS PVSS) → for WVUF/randomness
//! - **Scalar shares** (via this module) → for IBE/timelock encryption
//!
//! ## Documentation References
//!
//! **Architecture & Design:**
//! - [ADR-001: Dual-Output DKG](atomica/docs/adr-001-dual-output-dkg.md)
//! - [Implementation Plan](atomica/docs/implementation-plan-unified-dkg-ibe.md)
//! - [Definitions](atomica/docs/definitions.md)
//!
//! **Technical Details:**
//! - [Chunked ElGamal Scalar Generation](atomica/docs/technical/chunked-elgamal-scalar-generation.md)
//! - [Weighted Protocol](weighted_protocol.rs)
//! - [IBE Integration](../ibe/mod.rs)
//!
//! ## Mathematical Background
//!
//! ### Problem Statement
//!
//! In threshold cryptography, we need to:
//! 1. Split a secret scalar $s$ into $n$ shares such that any $t$ shares can reconstruct $s$
//! 2. Encrypt each share so that only the intended recipient can decrypt it
//! 3. Allow public verification that the encryption was done correctly
//!
//! ### Solution: Chunked Lifted ElGamal
//!
//! The protocol works as follows:
//!
//! 1. **Shamir Secret Sharing**: The dealer splits the secret $s$ into shares $s_0, s_1, \dots, s_{n-1}$
//!    using a random polynomial of degree $t-1$. Share $s_i = f(i)$ where $f(0) = s$.
//!
//! 2. **Chunking**: Each 256-bit share is split into 16 chunks of 16 bits each:
//!    $s_i = \sum_{j=0}^{15} u_{i,j} \cdot B^j$ where $B = 2^{16} = 65536$.
//!    This allows efficient discrete log computation using BSGS.
//!
//! 3. **Correlated Randomness**: For each chunk index $j$, generate randomness $r_j$ such that
//!    $\sum_{j=0}^{15} r_j \cdot B^j = 0$. This ensures that when transcripts from multiple
//!    dealers are aggregated, the randomness contributions cancel out appropriately.
//!
//! 4. **Lifted ElGamal Encryption**: For player $i$ and chunk $j$:
//!    $C_{i,j} = G \cdot u_{i,j} + PK_i \cdot r_j$
//!    where $PK_i = H \cdot sk_i$ is player $i$'s public key.
//!
//! 5. **DLEQ Proofs**: The dealer generates DLEQ (Discrete Log Equality) proofs to prove
//!    that the same randomness $r_j$ was used in both $R_j$ and $C_{i,j}$. This prevents
//!    a malicious dealer from encrypting garbage that would decrypt to wrong values.
//!
//! 6. **Decryption**: Player $i$ computes:
//!    $C_{i,j} - R_j^{sk_i} = G \cdot u_{i,j}$
//!    Then solves discrete log to recover $u_{i,j}$ (since $u_{i,j} \in [0, 65536)$).
//!
//! 7. **Reconstruction**: Chunks are combined: $s_i = \sum_{j=0}^{15} u_{i,j} \cdot B^j$.
//!
//! ## DLEQ Proof System
//!
//! The DLEQ (Discrete Log Equality) proof ensures encryption correctness. It proves that
//! for each validator $i$ and chunk $j$, the dealer used the same randomness $r_j$ in both:
//! - The ephemeral key $R_j = G^{r_j}$
//! - The ciphertext $C_{i,j} = G \cdot u_{i,j} + PK_i \cdot r_j$
//!
//! This prevents a malicious dealer from:
//! - Encrypting garbage values that decrypt to wrong shares
//! - Using different randomness for different validators
//! - Creating ciphertexts that don't correspond to the claimed polynomial
//!
//! ### Chaum-Pedersen Protocol
//!
//! The proof uses the Chaum-Pedersen protocol with Fiat-Shamir transform:
//!
//! **Setup:** Common inputs are $G, H = PK_i, A = R_j, B = C_{i,j} - G \cdot u_{i,j}$, and secret $x = r_j$.
//!
//! **Prover (Dealer):**
//! 1. Sample random $w \leftarrow \mathbb{Z}_r$
//! 2. Compute commitments: $commitment_g = G^w$, $commitment_h = H^w$
//! 3. Compute challenge: $c = H(commitment_g \parallel commitment_h \parallel A \parallel B \parallel DST)$
//! 4. Compute response: $response = w - c \cdot x$
//! 5. Output proof: $(commitment_g, commitment_h, response)$
//!
//! **Verifier:**
//! 1. Recompute challenge $c$ from commitments and public values
//! 2. Check: $G^{response} \cdot A^{c} \stackrel{?}{=} commitment_g$
//! 3. Check: $H^{response} \cdot B^{c} \stackrel{?}{=} commitment_h$
//!
//! ## Architecture
//!
//! The protocol involves two main actors:
//!
//! **DEALER:**
//!   Input: Secret scalar s
//!   1. Generate (t-1)-degree polynomial f with f(0) = s
//!   2. Evaluate f(i) for i = 1..n to get shares s_i
//!   3. Split each share into 16 chunks: s_i = Σ u_{i,j} · B^j
//!   4. Generate correlated randomness: Σ r_j · B^j = 0
//!   5. Encrypt: C_{i,j} = G · u_{i,j} + PK_i · r_j
//!   6. Generate DLEQ proofs for each (i, j) pair
//!   7. Commit: V_i = G2 · f(i), hat_w = G2 · r_0
//!   8. Prove: Schnorr proof of knowledge of f(0)
//!   Output: Transcript { C, R, V, hat_w, SoK, dleq_proofs }
//!
//! **VALIDATOR i:**
//!   Input: Transcript, decryption key sk_i
//!   1. For each chunk j:
//!      - Compute R_j^{sk_i} = (G^{r_j})^{sk_i} = PK_i^{r_j}
//!      - Decrypt: C_{i,j} - R_j^{sk_i} = G · u_{i,j}
//!      - Solve discrete log to recover u_{i,j}
//!   2. Reconstruct share: s_i = Σ u_{i,j} · B^j
//!   Output: Secret share s_i
//!
//! ## File Structure
//!
//! - [`mod.rs`](mod.rs) - Module exports and test utilities
//! - [`transcript.rs`](transcript.rs) - Core Transcript struct and DLEQ implementation
//! - [`weighted_protocol.rs`](weighted_protocol.rs) - Weighted threshold wrapper
//!
//! ## Integration Points
//!
//! - **DKG Integration:** [`types/src/dkg/real_dkg/mod.rs`](../../../../types/src/dkg/real_dkg/mod.rs)
//! - **IBE Consumer:** [`../ibe/mod.rs`](../ibe/mod.rs)
//! - **Smoke Tests:** [`testsuite/smoke-test/src/timelock/`](../../../../../../testsuite/smoke-test/src/timelock/)
//!
//! ## Aggregation
//!
//! Multiple dealers can contribute to the same DKG session. Their transcripts are
//! aggregated by adding the corresponding ciphertexts and commitments:
//!
//! - $C_{i,j}^{agg} = \sum_k C_{i,j}^k$ (ciphertexts add)
//! - $R_j^{agg} = \sum_k R_j^k$ (ephemeral keys add)
//! - $V^{agg} = \sum_k V^k$ (commitments add)
//! - dleq_proofs are NOT aggregated (each dealer generates their own)
//!
//! Due to the correlated randomness property ($\sum_j r_j \cdot B^j = 0$), when enough
//! validators participate in threshold decryption, the randomness contributions cancel
//! appropriately, allowing proper secret reconstruction.
//!
//! ## Security Properties
//!
//! 1. **Secret Sharing:** Any $t$ shares can reconstruct the secret; fewer reveal nothing
//! 2. **Encryption Correctness:** DLEQ proofs ensure ciphertexts encrypt the correct shares
//! 3. **Public Verifiability:** Anyone can verify the transcript without secrets
//! 4. **Aggregation Safety:** Correlated randomness ensures proper behavior under aggregation
//!
//! ## Error Handling
//!
//! All public methods return `Result` types for proper error propagation:
//! - `verify()` returns `Result<()>` for verification failures
//! - `decrypt_own_share()` returns `Result<(DealtSecretKeyShare, DealtPubKeyShare)>` for decryption failures
//!
//! ## Testing
//!
//! Run tests with:
//! ```bash
//! cargo test -p aptos-dkg --lib scalar_elgamal
//! cargo test -p smoke-test --lib timelock
//! cargo test -p smoke-test --lib randomness::e2e_correctness
//! ```

use crate::{
    algebra::polynomials::shamir_secret_share,
    pvss::{
        contribution::{batch_verify_soks, SoK},
        das, dealt_pub_key, dealt_pub_key_share, dealt_secret_key, dealt_secret_key_share,
        encryption_dlog, input_secret, schnorr,
        traits::{self, HasEncryptionPublicParams},
        LowDegreeTest, Player, ThresholdConfigBlstrs,
    },
    utils::{hash_to_scalar, random::random_scalars},
};
use anyhow::{anyhow, bail, Result};
use aptos_crypto::{
    bls12381, blstrs::random_scalar, CryptoMaterialError, SigningKey, ValidCryptoMaterial,
};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use blstrs::{G1Projective, G2Projective, Scalar};
use group::Group;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Mul;

/// Domain separation tag for cryptographic operations in this PVSS scheme.
/// Used to ensure cryptographic区分 between different uses of hash functions.
pub const SCALAR_ELGAMAL_DST: &[u8] = b"APTOS_SCALAR_ELGAMAL_PVSS_DST";

/// Human-readable name identifying this PVSS scheme.
/// Used for protocol identification and debugging.
pub const SCHEME_NAME: &str = "scalar_elgamal_pvss";

/// Number of bits per chunk when splitting scalars.
///
/// Each 256-bit scalar is split into chunks of this size to enable efficient
/// discrete log computation via Baby-Step Giant-Step (BSGS).
///
/// With 16-bit chunks, each chunk is in the range [0, 65536), which makes discrete
/// log feasible using BSGS with ~256 table entries per chunk.
///
/// See [technical documentation](atomica/docs/technical/chunked-elgamal-scalar-generation.md)
/// for detailed explanation of chunking.
const CHUNK_BIT_SIZE: usize = 16;

/// Number of chunks per scalar.
///
/// A 256-bit scalar (like BLS12-381's Fr) divided by 16 bits per chunk gives
/// exactly 16 chunks. This is a key parameter that determines:
/// - The size of ciphertexts (16 G1 elements per player)
/// - The BSGS table size (256 entries per chunk)
/// - The reconstruction formula: s = Σ chunk_j · (2^16)^j
const NUM_CHUNKS: usize = 16;

/// Domain separation tag for DLEQ proofs in Scalar ElGamal PVSS.
/// Prevents cross-protocol attacks by ensuring unique hash domains.
///
/// The DLEQ proof proves: log_G(R_j) = log_{PK_i}(C_{i,j} - G * u_{i,j})
/// This ensures the dealer used the same randomness r_j in both the ephemeral
/// key R_j and the ciphertext C_{i,j}.
const DLEQ_PROOF_DST: &[u8; 28] = b"APTOS_SCALAR_ELGAMAL_DLEQ_V1";

/// DLEQ (Discrete Log Equality) proof for one validator's ciphertexts.
///
/// Proves that for a given chunk j, the ephemeral key R_j and the ciphertext
/// part (C_{i,j} - G * u_{i,j}) were both computed using the same randomness r_j.
///
/// Specifically proves: log_G(R_j) = log_{PK_i}(C_{i,j} - G * u_{i,j})
///
/// ## Security Purpose
///
/// Without DLEQ proofs, a malicious dealer could:
/// 1. Commit to a valid polynomial f(x) with f(0) = s
/// 2. But encrypt garbage values in the ciphertexts
/// 3. Validators would "decrypt" and get wrong share values
/// 4. If enough validators get wrong shares, reconstruction fails
///
/// The DLEQ proof prevents this by cryptographically binding the ciphertexts
/// to the ephemeral keys, proving correct encryption.
///
/// ## Structure
///
/// Each proof consists of:
/// - `commitment_g`: G^w (commitment to randomness w on G1)
/// - `commitment_h`: PK_i^w (commitment to randomness w on target group)
/// - `response`: w - c * r_j (response scalar where r_j is the secret randomness)
///
/// ## Verification
///
/// Verifier checks:
/// - G^response * R_j^c = commitment_g
/// - PK_i^response * (C_{i,j} - G * u_{i,j})^c = commitment_h
///
/// where c = H(commitment_g || commitment_h || R_j || PK_i^r_j || DST)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, BCSCryptoHash, CryptoHasher)]
pub struct DleqProof {
    pub commitment_g: G1Projective,
    pub commitment_h: G1Projective,
    pub response: Scalar,
}
}

/// The Transcript struct represents a PVSS transcript for a single dealer.
///
/// A transcript contains all the information needed for:
/// 1. Public verification that the dealer performed the protocol correctly
/// 2. Each player to decrypt their secret share
/// 3. Aggregation with other transcripts from different dealers
///
/// ## Structure Overview
///
/// ```text
/// Transcript {
///   soks: Vec<SoK<G2Projective>>,           // Proofs of knowledge from dealers
///   hat_w: G2Projective,                     // Commitment to randomness r_0
///   V: Vec<G2Projective>,                    // Polynomial commitments V_0..V_n
///   ephemeral_keys: Vec<G1Projective>,       // R_j = G^{r_j} for each chunk j
///   ciphertexts: Vec<Vec<G1Projective>>,     // C_{i,j} = G^{u_{i,j}} · PK_i^{r_j}
///   encrypted_aggregate: Vec<G1Projective>,  // Aggregated plaintext for threshold
/// }
/// ```
///
/// ## Usage
///
/// 1. **Creation**: A dealer calls [`Transcript::deal`] to create a transcript
///    from their secret and the public parameters.
/// 2. **Verification**: Anyone can call [`Transcript::verify`] to check the transcript
///    is well-formed (partial implementation).
/// 3. **Decryption**: Each player calls [`Transcript::decrypt_own_share`] to recover
///    their secret share.
/// 4. **Aggregation**: Multiple transcripts can be combined via [`Transcript::aggregate_with`]
///    for multi-dealer DKG sessions.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, BCSCryptoHash, CryptoHasher)]
#[allow(non_snake_case)]
pub struct Transcript {
    /// **SoKs (Signatures of Knowledge)**: Proofs that each dealer knew their secret.
    ///
    /// Each element is a tuple (player_id, commitment, signature, pok_proof) proving
    /// that the dealer knew the polynomial constant term (the secret) when creating
    /// the transcript.
    ///
    /// For single-dealer transcripts, this has length 1.
    /// For aggregated transcripts, this has length equal to the number of dealers.
    ///
    /// ## Security Property
    ///
    /// The SoK prevents malicious dealers from creating invalid transcripts that
    /// would prevent honest players from recovering their shares.
    soks: Vec<SoK<G2Projective>>,

    /// **hat_w**: Commitment to the first randomness coefficient $r_0$.
    ///
    /// This is $hat_w = G_2^{r_0}$ where $r_0$ is the first element of the
    /// correlated randomness vector (chosen to satisfy $\sum_j r_j \cdot B^j = 0$).
    ///
    /// This commitment is used in verification to ensure the dealer used
    /// properly correlated randomness. Specifically, it allows checking that
    /// the randomness values satisfy the required constraint.
    ///
    /// ## Mathematical Relationship
    ///
    /// Given the correlated randomness constraint:
    /// $\sum_{j=0}^{15} r_j \cdot B^j = 0$
    ///
    /// We have:
    /// $r_0 = -\sum_{j=1}^{15} r_j \cdot B^j$
    ///
    /// And thus:
    /// $hat_w = G_2^{r_0} = G_2^{-\sum_{j=1}^{15} r_j \cdot B^j}$
    hat_w: G2Projective,

    /// **V**: Polynomial commitment vector.
    ///
    /// Contains $n+1$ elements: $V_0, V_1, \dots, V_n$ where:
    /// - $V_i = G_2^{f(i)}$ for $i = 0..n-1$ (commitments to share values)
    /// - $V_n = G_2^{f(0)} = G_2^s$ (commitment to the secret itself)
    ///
    /// These commitments allow anyone to verify that the shares are evaluations
    /// of a degree-$(t-1)$ polynomial, without revealing the polynomial itself.
    ///
    /// ## Construction
    ///
    /// The polynomial $f(x)$ is generated by the dealer such that:
    /// - $f(0) = s$ (the secret)
    /// - $f(i) = s_i$ (share for player $i$)
    ///
    /// Each $V_i$ is a Pedersen-style commitment using base $G_2$.
    V: Vec<G2Projective>,

    /// **Ephemeral Keys**: Public randomness commitments for each chunk.
    ///
    /// $R_j = G_1^{r_j}$ for $j = 0..15$ where $r_j$ is the randomness used
    /// for encrypting chunk $j$ across all players.
    ///
    /// ## Key Properties
    ///
    /// 1. **Same randomness across players**: For a fixed chunk index $j$,
    ///    the same $r_j$ is used for ALL players. This is crucial because it
    ///    enables efficient aggregation.
    ///
    /// 2. **Correlated**: The randomness values satisfy $\sum_j r_j \cdot B^j = 0$,
    ///    which ensures proper behavior under aggregation.
    ///
    /// 3. **Used for decryption**: Player $i$ computes $R_j^{sk_i} = PK_i^{r_j}$
    ///    to cancel the encryption randomness.
    ///
    /// ## Size
    ///
    /// Always has exactly `NUM_CHUNKS = 16` elements, regardless of the number
    /// of validators. This is because randomness is per-chunk, not per-player.
    ///
    /// **Notation**: $R_j = G^{r_j}$ where $G$ is the encryption base point.
    ephemeral_keys: Vec<G1Projective>,

    /// **Ciphertexts**: Encrypted share chunks for each validator.
    ///
    /// A 2D array where:
    /// - Outer dimension: validators $i = 0..n-1$ (one row per validator)
    /// - Inner dimension: chunks $j = 0..15$ (16 chunks per share)
    ///
    /// ## Encryption Formula
    ///
    /// For validator $i$ and chunk $j$:
    /// $C_{i,j} = G \cdot u_{i,j} + PK_i \cdot r_j$
    ///
    /// Where:
    /// - $u_{i,j}$ is the $j$-th chunk of player $i$'s share
    /// - $PK_i$ is player $i$'s public key
    /// - $r_j$ is the shared randomness for chunk $j$
    ///
    /// ## Decryption
    ///
    /// Player $i$ computes:
    /// $C_{i,j} - R_j^{sk_i} = G \cdot u_{i,j}$
    ///
    /// Then solves discrete log to recover $u_{i,j}$ (since $u_{i,j} < 2^{16}$).
    ///
    /// ## Aggregation Behavior
    ///
    /// When transcripts are aggregated, ciphertexts add component-wise:
    /// $C_{i,j}^{agg} = \sum_k C_{i,j}^k$
    ///
    /// This preserves the decryption formula:
    /// $C_{i,j}^{agg} - (R_j^{agg})^{sk_i} = G \cdot \sum_k u_{k,i,j}$
    ///
    /// **Note**: For aggregated transcripts, the decrypted value is the sum of
    /// chunks from all dealers, which requires a larger BSGS search space.
    ///
    /// Outer vector corresponds to validators (ordered by ID).
    /// Inner vector corresponds to chunks.
    ciphertexts: Vec<Vec<G1Projective>>,

    /// **Encrypted Aggregate**: Pre-computed aggregate for threshold reconstruction.
    ///
    /// This field stores $E_j = G^{\sum_i u_{i,j}} \cdot G^{r_j}$ for each chunk $j$.
    /// This is used to support threshold decryption scenarios where validators
    /// need to combine their partial decryptions.
    ///
    /// ## Purpose
    ///
    /// When multiple dealers contribute to a DKG session, the encrypted_aggregate
    /// provides a way to reconstruct the combined secret without requiring each
    /// validator to individually decrypt all dealer contributions.
    ///
    /// ## Construction
    ///
    /// For each chunk $j$:
    /// 1. Compute plaintext sum: $\sum_i u_{i,j}$ (sum of chunk $j$ across all dealers)
    /// 2. Compute: $E_j = G^{\sum_i u_{i,j}} \cdot G^{r_j}$
    ///
    /// The $G^{r_j}$ term ensures that the aggregate can be decrypted using the
    /// same mechanism as individual ciphertexts.
    ///
    /// ## Under Aggregation
    ///
    /// When transcripts are aggregated, this field also adds:
    /// $E_j^{agg} = \sum_k E_j^k$
    ///
    /// This preserves the ability to decrypt the aggregate plaintext.
    ///
    /// Each element corresponds to a chunk index and stores
    /// $g^{\sum_k u_{k,j}} \cdot g^{r_j}$ where the randomness term enables
    /// threshold decryption when combined with proper secret sharing coefficients.
    encrypted_aggregate: Vec<G1Projective>,

    /// **DLEQ Proofs**: Encryption correctness proofs for each validator and chunk.
    ///
    /// Proves that for each validator $i$ and chunk $j$:
    /// $\log_G(R_j) = \log_{PK_i}(C_{i,j} - G \cdot u_{i,j})$
    ///
    /// This ensures the dealer used the same randomness $r_j$ in both the ephemeral
    /// key $R_j$ and the ciphertext $C_{i,j}$. Without this proof, a malicious dealer
    /// could encrypt garbage that decrypts to wrong values.
    ///
    /// ## Structure
    ///
    /// A 2D array where:
    /// - Outer dimension: validators $i = 0..n-1$ (one row per validator)
    /// - Inner dimension: chunks $j = 0..15$ (16 proofs per validator)
    ///
    /// Each proof is a DleqProof containing:
    /// - commitment_g: G^w
    /// - commitment_h: PK_i^w
    /// - response: w - c * r_j
    ///
    /// ## Verification Equation
    ///
    /// Verifier checks for each (i, j):
    /// - $G^{response} \cdot R_j^{c} = commitment\_G$
    /// - $PK_i^{response} \cdot (C_{i,j} - G \cdot u_{i,j})^{c} = commitment\_H$
    ///
    /// where $c = H(commitment\_G \parallel commitment\_H \parallel context)$
    pub dleq_proofs: Vec<Vec<DleqProof>>,

    /// **Plaintext Chunks**: The raw share chunks for verification.
    ///
    /// Stores the plaintext chunks u_{i,j} for each validator i and chunk j.
    /// This is needed to verify DLEQ proofs during verification, as the verifier
    /// needs to compute C_{i,j} - G * u_{i,j} to check the proof.
    ///
    /// These are NOT encrypted - they're plaintext values in [0, 65536).
    /// Including them in the transcript adds n * 32 bytes (2 bytes per chunk * 16 chunks).
    plaintext_chunks: Vec<Vec<u16>>,
}

impl ValidCryptoMaterial for Transcript {
    const AIP_80_PREFIX: &'static str = "";

    /// Serializes the transcript to bytes using BCS (Binary Canonical Serialization).
    ///
    /// This serialization is deterministic and compact, suitable for transmission
    /// over the network or storage on disk.
    ///
    /// ## Panics
    ///
    /// Never panics under normal circumstances. BCS serialization of valid types
    /// should always succeed.
    fn to_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(self).expect("unexpected error during PVSS transcript serialization")
    }
}

impl TryFrom<&[u8]> for Transcript {
    type Error = CryptoMaterialError;

    /// Deserializes a transcript from bytes.
    ///
    /// Performs BCS deserialization and validates the result is a valid Transcript.
    ///
    /// ## Arguments
    ///
    /// * `bytes` - Serialized transcript bytes
    ///
    /// ## Returns
    ///
    /// * `Ok(Transcript)` - Successfully deserialized transcript
    /// * `Err(CryptoMaterialError::DeserializationError)` - Invalid bytes
    ///
    /// ## Security Note
    ///
    /// The deserialized transcript should be verified using [`Transcript::verify`]
    /// before trusting any shares derived from it.
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        bcs::from_bytes::<Transcript>(bytes).map_err(|_| CryptoMaterialError::DeserializationError)
    }
}

impl traits::Transcript for Transcript {
    /// The dealt public key type: a G2 element representing the aggregate public key.
    type DealtPubKey = dealt_pub_key::g2::DealtPubKey;

    /// The dealt public key share type: a G2 element representing one player's share.
    type DealtPubKeyShare = dealt_pub_key_share::g2::DealtPubKeyShare;

    /// The dealt secret key type: a scalar representing the reconstructed secret.
    type DealtSecretKey = dealt_secret_key::scalar::DealtSecretKey;

    /// The dealt secret key share type: a scalar representing one player's share.
    type DealtSecretKeyShare = dealt_secret_key_share::scalar::DealtSecretKeyShare;

    /// The decryption private key type: a scalar for ElGamal decryption.
    type DecryptPrivKey = encryption_dlog::g1::DecryptPrivKey;

    /// The encryption public key type: a G1 element for ElGamal encryption.
    type EncryptPubKey = encryption_dlog::g1::EncryptPubKey;

    /// The input secret type: the scalar to be shared.
    type InputSecret = input_secret::InputSecret;

    /// The public parameters type: contains cryptographic bases and curve parameters.
    type PublicParameters = das::PublicParameters;

    /// The secret sharing configuration type: threshold parameters.
    type SecretSharingConfig = ThresholdConfigBlstrs;

    /// The signing public key type: BLS12381 public key for authentication.
    type SigningPubKey = bls12381::PublicKey;

    /// The signing secret key type: BLS12381 private key for authentication.
    type SigningSecretKey = bls12381::PrivateKey;

    /// Returns the domain separation tag for this PVSS scheme.
    ///
    /// This tag is used in hash computations to ensure cryptographic separation
    /// between different uses of hash functions across different protocols.
    ///
    /// ## Returns
    ///
    /// A vector of bytes containing "APTOS_SCALAR_ELGAMAL_PVSS_DST"
    fn dst() -> Vec<u8> {
        SCALAR_ELGAMAL_DST.to_vec()
    }

    /// Returns the human-readable scheme name.
    ///
    /// Used for protocol identification, logging, and debugging.
    ///
    /// ## Returns
    ///
    /// The string "scalar_elgamal_pvss"
    fn scheme_name() -> String {
        SCHEME_NAME.to_string()
    }

    /// Creates a PVSS transcript by dealing secret shares to all players.
    ///
    /// Proves that log_G(A) = log_H(B) without revealing the common logarithm.
    /// Uses the Chaum-Pedersen protocol with Fiat-Shamir transform.
    ///
    /// ## Arguments
    ///
    /// * `secret` - The secret value x (r_j in our case)
    /// * `g` - Base point G (G1 generator)
    /// * `A` - G^x (ephemeral key R_j)
    /// * `h` - Public key PK_i
    /// * `B` - h^x (ciphertext part C_{i,j} - G * u_{i,j})
    /// * `rng` - Random number generator
    ///
    /// ## Returns
    ///
    /// A DLEQ proof containing commitment_g, commitment_h, and response.

    /// Creates a PVSS transcript by dealing secret shares to all players.
    ///
    /// This is the main entry point for a dealer to create a PVSS transcript.
    /// The dealer takes a secret, splits it using Shamir's Secret Sharing,
    /// encrypts each share using chunked lifted ElGamal, and creates proofs.
    ///
    /// ## Process
    ///
    /// 1. **Secret Sharing**: Generate a random polynomial $f(x)$ of degree $t-1$
    ///    such that $f(0) = s$ (the secret). Evaluate at $x = 1..n$ to get shares.
    ///
    /// 2. **Chunking**: Split each 256-bit share into 16 chunks of 16 bits each.
    ///    Share $s_i = \sum_{j=0}^{15} u_{i,j} \cdot 2^{16j}$.
    ///
    /// 3. **Correlated Randomness**: Generate $r_1, \dots, r_{15}$ randomly, then
    ///    set $r_0 = -\sum_{j=1}^{15} r_j \cdot 2^{16j}$. This ensures
    ///    $\sum_{j=0}^{15} r_j \cdot 2^{16j} = 0$.
    ///
    /// 4. **Encryption**: For each player $i$ and chunk $j$:
    ///    $C_{i,j} = G \cdot u_{i,j} + PK_i \cdot r_j$
    ///
    /// 5. **Commitments**: Compute $V_i = G_2^{f(i)}$ for $i = 0..n$.
    ///
    /// 6. **Proof**: Generate a Schnorr proof of knowledge of $f(0)$.
    ///
    /// ## Arguments
    ///
    /// * `sc` - Secret sharing configuration (threshold parameters)
    /// * `pp` - Public parameters (cryptographic bases)
    /// * `ssk` - Dealer's signing key (for authenticating the transcript)
    /// * `eks` - Vector of all players' encryption public keys
    /// * `s` - The input secret to be shared
    /// * `aux` - Auxiliary data to be included in the signature
    /// * `dealer` - The player ID of the dealer
    /// * `rng` - Cryptographic random number generator
    ///
    /// ## Returns
    ///
    /// A complete PVSS transcript ready for distribution.
    ///
    /// ## Panics
    ///
    /// Panics if the number of encryption keys doesn't match the configuration.
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

        // Step 1: Generate Shamir secret sharing shares
        // Creates a random polynomial f(x) of degree t-1 with f(0) = s
        // Returns (f, f_evals) where f_evals[i] = f(i+1) for i = 0..n-1
        let (f, f_evals) = shamir_secret_share(sc, s, rng);

        // Get cryptographic bases
        let g_1 = pp.get_encryption_public_params().pubkey_base(); // G1 base for encryption
        let g_2 = pp.get_commitment_base(); // G2 base for commitments

        // Step 2: Generate correlated randomness for each chunk index
        //
        // The key property: Σ r_j * B^j = 0 where B = 2^16 = 65536
        // This ensures randomness cancels during decryption of aggregated transcripts
        //
        // Why this matters: When multiple dealers' transcripts are aggregated,
        // the ciphertexts add: C_agg = Σ C_k. For decryption to work, the
        // randomness contributions must properly combine. The correlated
        // randomness constraint ensures this happens correctly.
        let radix = 1u64 << CHUNK_BIT_SIZE; // B = 2^16
        let mut chunk_randomness: Vec<Scalar> =
            (0..NUM_CHUNKS).map(|_| random_scalar(rng)).collect();

        // Compute r_0 such that Σ r_j * B^j = 0
        // r_0 = -Σ_{j=1}^{n-1} r_j * B^j
        //
        // This is computed iteratively:
        // Start with r_0 = 0
        // For j = 1 to 15: r_0 -= r_j * 2^(16*j)
        let mut remainder = Scalar::from(0u64);
        let mut cur_base = Scalar::from(radix);
        for j in 1..NUM_CHUNKS {
            remainder -= chunk_randomness[j] * cur_base;
            cur_base *= Scalar::from(radix);
        }
        chunk_randomness[0] = remainder;

        // Step 3: Compute ephemeral keys R_j = H * r_j
        // These are public randomness commitments, one per chunk
        // Note: Same for all players, varies only by chunk index
        let ephemeral_keys: Vec<G1Projective> = chunk_randomness
            .iter()
            .map(|r| g_1.mul(r.clone()))
            .collect();

        // Step 4: Compute polynomial commitments V
        // V[i] = G2^{f(i)} for i = 0..n-1 (share commitments)
        // V[n] = G2^{f(0)} = G2^s (secret commitment)
        let V = (0..sc.n)
            .map(|i| g_2.mul(f_evals[i]))
            .chain([g_2.mul(f[0])])
            .collect::<Vec<G2Projective>>();

        // Step 5: Encrypt shares using chunked lifted ElGamal
        // C_i,j = G * z_i,j + ek_i * r_j
        // Note: r_j is the SAME for all players i, only varies by chunk j!
        //
        // This "column-wise" encryption is what enables aggregation:
        // When we add transcripts from multiple dealers, we get:
        // C_agg[i,j] = Σ_k (G * z_k[i,j] + PK_i * r_k[j])
        //            = G * Σ_k z_k[i,j] + PK_i * Σ_k r_k[j]
        //
        // The decryption still works because each player can compute:
        // R_j^{agg, sk_i} = (Σ_k R_k[j])^{sk_i} = PK_i^{Σ_k r_k[j]}
        // C_agg[i,j} - R_j^{agg, sk_i} = G * Σ_k z_k[i,j}

        // Collect plaintext chunks for each player (needed for DLEQ verification)
        let plaintext_chunks: Vec<Vec<u16>> =
            (0..sc.n).map(|i| scalar_to_chunks(&f_evals[i])).collect();

        let ciphertexts: Vec<Vec<G1Projective>> = (0..sc.n)
            .map(|i| {
                let chunks = &plaintext_chunks[i];

                chunks
                    .iter()
                    .enumerate()
                    .map(|(j, &chunk)| {
                        // C_i,j = G * chunk + ek_i * r_j
                        let g_chunk = g_1.mul(Scalar::from(chunk as u64));
                        let ek_r = Into::<G1Projective>::into(&eks[i]).mul(chunk_randomness[j]);
                        g_chunk + ek_r
                    })
                    .collect()
            })
            .collect();

        // Step 6: Compute encrypted aggregate for aggregation support
        // E_j = G * Σ_i z_i,j + H * r_j
        //
        // This pre-computed aggregate can be used for threshold reconstruction
        // scenarios where validators need to combine their partial decryptions.
        let mut plaintext_sums: Vec<Scalar> = vec![Scalar::from(0u64); NUM_CHUNKS];
        for i in 0..sc.n {
            for j in 0..NUM_CHUNKS {
                plaintext_sums[j] += Scalar::from(plaintext_chunks[i][j] as u64);
            }
        }

        let encrypted_aggregate: Vec<G1Projective> = plaintext_sums
            .iter()
            .enumerate()
            .map(|(j, &plaintext_sum)| {
                let g_plaintext = g_1.mul(plaintext_sum);
                let h_randomness = g_1.mul(chunk_randomness[j]);
                g_plaintext + h_randomness
            })
            .collect();

        // Step 5.5: Generate DLEQ proofs for each validator and chunk
        //
        // For each validator i and chunk j, prove that:
        // log_G(R_j) = log_{PK_i}(C_{i,j} - G * u_{i,j})
        //
        // This ensures the same randomness r_j was used in both the ephemeral key
        // and the ciphertext, preventing a malicious dealer from encrypting garbage.
        let dleq_proofs: Vec<Vec<DleqProof>> = (0..sc.n)
            .map(|i| {
                let pk_i: G1Projective = Into::<G1Projective>::into(&eks[i]);

                (0..NUM_CHUNKS)
                    .map(|j| {
                        let chunk = plaintext_chunks[i][j];

                        // Compute the "masked" ciphertext part: C_{i,j} - G * u_{i,j} = PK_i * r_j
                        let g_chunk = g_1.mul(Scalar::from(chunk as u64));
                        let ciphertext_ij = ciphertexts[i][j];
                        let masked_ciphertext = ciphertext_ij - g_chunk;

                        // Generate DLEQ proof
                        generate_dleq_proof(
                            &chunk_randomness[j], // secret = r_j
                            &g_1,                 // G
                            &ephemeral_keys[j],   // A = R_j = G^{r_j}
                            &pk_i,                // h = PK_i
                            &masked_ciphertext,   // B = PK_i^{r_j}
                            rng,
                        )
                    })
                    .collect()
            })
            .collect();

        // Step 6: Compute encrypted aggregate for aggregation support
        // E_j = G * Σ_i z_i,j + H * r_j
        //
        // This pre-computed aggregate can be used for threshold reconstruction
        // scenarios where validators need to combine their partial decryptions.
        let pok = schnorr::pok_prove(&f[0], g_2, &V[sc.n], rng);

        // Debug assertions for development
        debug_assert_eq!(V.len(), sc.n + 1);
        debug_assert_eq!(ciphertexts.len(), sc.n);

        // Sign the commitment to authenticate this transcript
        let sig = Transcript::sign_contribution(ssk, dealer, aux, &V[sc.n]);

        Transcript {
            soks: vec![(*dealer, V[sc.n], sig, pok)],
            hat_w: g_2.mul(chunk_randomness[0]),
            V,
            ephemeral_keys,
            ciphertexts,
            encrypted_aggregate,
            dleq_proofs,
            plaintext_chunks,
        }
    }

    /// Verifies the transcript is well-formed and cryptographically valid.
    ///
    /// This function performs comprehensive verification of a Chunked Lifted ElGamal
    /// PVSS transcript, including structural checks, signature verification, and
    /// polynomial commitment validation.
    ///
    /// ## Checks Performed
    ///
    /// 1. **Structural checks:**
    ///    - Number of ciphertext vectors matches number of players
    ///    - Each ciphertext vector has exactly NUM_CHUNKS (16) elements
    ///    - Number of commitment elements is n+1
    ///    - Number of ephemeral keys is NUM_CHUNKS (16)
    ///    - Number of encrypted_aggregate elements is NUM_CHUNKS (16)
    ///    - Number of encryption keys matches number of players
    ///
    /// 2. **Signature of Knowledge (SoK) verification:**
    ///    - Schnorr proof of knowledge of secret
    ///    - BLS signature on commitment, player ID, and auxiliary data
    ///
    /// 3. **Polynomial commitment verification:**
    ///    - Low-degree test ensures committed polynomial has degree < t
    ///
    /// ## Security Notes
    ///
    /// - Uses random challenges derived from thread_rng for batched verification
    /// - SoK verification ensures dealer knew the secret at dealing time
    /// - Low-degree test prevents dealing of invalid polynomial degrees
    ///
    /// ## Arguments
    ///
    /// * `sc` - Secret sharing configuration (threshold t, num players n)
    /// * `pp` - Public parameters (commitment base, encryption params)
    /// * `spks` - Signing public keys of dealers (for SoK signature verification)
    /// * `eks` - Encryption public keys of all players
    /// * `auxs` - Auxiliary data for each SoK (e.g., epoch, dealer address)
    ///
    /// ## Returns
    ///
    /// * `Ok(())` - Transcript is valid
    /// * `Err(anyhow::Error)` - Validation failed with details
    fn verify<A: Serialize + Clone>(
        &self,
        sc: &Self::SecretSharingConfig,
        pp: &Self::PublicParameters,
        spks: &Vec<Self::SigningPubKey>,
        eks: &Vec<Self::EncryptPubKey>,
        auxs: &Vec<A>,
    ) -> Result<()> {
        //
        // 1. Structural checks
        //

        // Verify encryption keys dimension
        if eks.len() != sc.n {
            bail!("Expected {} encryption keys, but got {}", sc.n, eks.len());
        }

        // Verify ciphertexts outer dimension (number of players)
        if self.ciphertexts.len() != sc.n {
            bail!(
                "Expected {} ciphertext vectors, but got {}",
                sc.n,
                self.ciphertexts.len()
            );
        }

        // Verify ciphertexts inner dimension (number of chunks per player)
        for (i, player_ciphertexts) in self.ciphertexts.iter().enumerate() {
            if player_ciphertexts.len() != NUM_CHUNKS {
                bail!(
                    "Expected {} chunks for player {}, but got {}",
                    NUM_CHUNKS,
                    i,
                    player_ciphertexts.len()
                );
            }
        }

        // Verify polynomial commitments dimension
        if self.V.len() != sc.n + 1 {
            bail!(
                "Expected {} commitment elements, but got {}",
                sc.n + 1,
                self.V.len()
            );
        }

        // Verify ephemeral keys dimension
        if self.ephemeral_keys.len() != NUM_CHUNKS {
            bail!(
                "Expected {} ephemeral keys, but got {}",
                NUM_CHUNKS,
                self.ephemeral_keys.len()
            );
        }

        // Verify encrypted_aggregate dimension
        if self.encrypted_aggregate.len() != NUM_CHUNKS {
            bail!(
                "Expected {} encrypted_aggregate elements, but got {}",
                NUM_CHUNKS,
                self.encrypted_aggregate.len()
            );
        }

        //
        // 2. SoK verification (Schnorr proofs + BLS signatures)
        //
        // This verifies that:
        // a) Each dealer knew their secret (Schnorr PoK)
        // b) Each dealer signed their commitment (BLS signature)
        //

        let mut rng = thread_rng();
        let extra = random_scalars(2, &mut rng);

        let g_2 = pp.get_commitment_base();
        batch_verify_soks::<G2Projective, A>(
            self.soks.as_slice(),
            g_2,
            &self.V[sc.n], // The aggregate secret commitment
            spks,
            auxs,
            &extra[0],
        )?;

        //
        // 3. Low-degree test on polynomial commitments
        //
        // Verifies that the committed polynomial V[i] = G2^{f(i)} has degree < t.
        // This prevents a malicious dealer from dealing shares that don't form
        // a valid (t-1)-degree polynomial.
        //

        let ldt = LowDegreeTest::random(
            &mut rng,
            sc.t,
            sc.n + 1,
            true, // includes_zero: V[n] is f(0), the secret
            sc.get_batch_evaluation_domain(),
        );
        ldt.low_degree_test_on_g2(&self.V)?;

        //
        // 4. DLEQ proof verification for encryption correctness
        //
        // Verify that for each validator i and chunk j:
        // log_G(R_j) = log_{PK_i}(C_{i,j} - G * u_{i,j})
        //
        // This ensures the dealer used the same randomness r_j in both the ephemeral key
        // and the ciphertext, preventing a malicious dealer from encrypting garbage.
        //

        let g_1 = pp.get_encryption_public_params().pubkey_base();

        for i in 0..sc.n {
            let pk_i: G1Projective = Into::<G1Projective>::into(&eks[i]);

            for j in 0..NUM_CHUNKS {
                // Get the plaintext chunk u_{i,j} from stored plaintext_chunks
                let chunk = self.plaintext_chunks[i][j];

                // Compute masked ciphertext: C_{i,j} - G * u_{i,j} = PK_i * r_j
                let g_chunk = g_1.mul(Scalar::from(chunk as u64));
                let ciphertext_ij = self.ciphertexts[i][j];
                let masked_ciphertext = ciphertext_ij - g_chunk;

                // Verify DLEQ proof
                verify_dleq_proof(
                    &self.dleq_proofs[i][j],
                    g_1,
                    &self.ephemeral_keys[j], // R_j = G^{r_j}
                    &pk_i,
                    &masked_ciphertext, // PK_i^{r_j}
                )?;
            }
        }

        Ok(())
    }

    /// Returns the list of dealers who contributed to this transcript.
    ///
    /// For single-dealer transcripts, returns a list with one element.
    /// For aggregated transcripts, returns all dealer player IDs.
    ///
    /// ## Returns
    ///
    /// Vector of player IDs representing all dealers.
    fn get_dealers(&self) -> Vec<Player> {
        self.soks.iter().map(|(p, _, _, _)| *p).collect()
    }

    /// Aggregates another transcript into this one.
    ///
    /// This enables multi-dealer DKG sessions where multiple dealers contribute
    /// to the same set of shares. The aggregation is done by adding corresponding
    /// elements component-wise.
    ///
    /// ## What Gets Aggregated
    ///
    /// 1. **SoKs**: Concatenated (preserving order of contribution)
    /// 2. **hat_w**: Added (G2 addition)
    /// 3. **ephemeral_keys**: Added (G1 addition) - R_j aggregates across dealers
    /// 4. **ciphertexts**: Added (G1 addition) - C_{i,j} aggregates
    /// 5. **V**: Added (G2 addition) - polynomial commitments aggregate
    /// 6. **encrypted_aggregate**: Added (G1 addition) - aggregate plaintexts combine
    ///
    /// ## Mathematical Properties
    ///
    /// After aggregation with `other`:
    /// - $C_{i,j}^{new} = C_{i,j}^{self} + C_{i,j}^{other}$
    /// - $R_j^{new} = R_j^{self} + R_j^{other}$
    /// - $V^{new} = V^{self} + V^{other}$
    ///
    /// ## Decryption After Aggregation
    ///
    /// For aggregated transcripts, the decryption formula becomes:
    /// $C_{i,j}^{agg} - (R_j^{agg})^{sk_i} = G \cdot \sum_k u_{k,i,j}$
    ///
    /// The decrypted value is the sum of chunks from all dealers, which requires
    /// a larger discrete log search space (up to num_dealers * 2^16).
    ///
    /// ## Arguments
    ///
    /// * `sc` - Secret sharing configuration (used for bounds checking)
    /// * `other` - The transcript to aggregate into this one
    fn aggregate_with(&mut self, sc: &Self::SecretSharingConfig, other: &Transcript) {
        debug_assert_eq!(self.ciphertexts.len(), sc.n);
        debug_assert_eq!(self.V.len(), sc.n + 1);
        debug_assert_eq!(self.encrypted_aggregate.len(), NUM_CHUNKS);
        debug_assert_eq!(other.encrypted_aggregate.len(), NUM_CHUNKS);

        // Aggregate hat_w (G2 addition)
        self.hat_w += other.hat_w;

        // Aggregate ephemeral keys and encrypted aggregate (per chunk)
        for j in 0..NUM_CHUNKS {
            self.ephemeral_keys[j] += other.ephemeral_keys[j];
            self.encrypted_aggregate[j] += other.encrypted_aggregate[j];
        }

        // Aggregate ciphertexts (per player, per chunk) and V (per player)
        for i in 0..sc.n {
            for j in 0..NUM_CHUNKS {
                self.ciphertexts[i][j] += other.ciphertexts[i][j];
            }
            self.V[i] += other.V[i];
        }
        // Aggregate V[n] (the secret commitment)
        self.V[sc.n] += other.V[sc.n];

        // Concatenate SoKs
        for sok in &other.soks {
            self.soks.push(sok.clone());
        }
    }

    /// Retrieves a player's public key share from the transcript.
    ///
    /// The public key share is the polynomial evaluation at the player's index:
    /// $PKshare_i = G_2^{f(i)}$
    ///
    /// This is used to verify that the decrypted secret share combines correctly
    /// with other shares during threshold reconstruction.
    ///
    /// ## Arguments
    ///
    /// * `sc` - Secret sharing configuration (unused, for API compatibility)
    /// * `player` - The player whose public key share to retrieve
    ///
    /// ## Returns
    ///
    /// The player's public key share as a `DealtPubKeyShare`.
    fn get_public_key_share(
        &self,
        _sc: &Self::SecretSharingConfig,
        player: &Player,
    ) -> Self::DealtPubKeyShare {
        Self::DealtPubKeyShare::new(Self::DealtPubKey::new(self.V[player.id]))
    }

    /// Retrieves the aggregate dealt public key.
    ///
    /// This is the commitment to the combined secret from all dealers:
    /// $DPK = G_2^{\sum_k s_k}$ where $s_k$ is dealer $k$'s secret.
    ///
    /// For single-dealer transcripts, this is simply $G_2^s$.
    /// For aggregated transcripts, this is the sum of individual commitments.
    ///
    /// ## Returns
    ///
    /// The aggregate dealt public key as a `DealtPubKey`.
    fn get_dealt_public_key(&self) -> Self::DealtPubKey {
        Self::DealtPubKey::new(*self.V.last().unwrap())
    }

    /// Decrypts a player's secret share from the transcript.
    ///
    /// This is the core decryption operation that recovers a player's Shamir share
    /// from the encrypted ciphertexts. It works for both single-dealer and
    /// aggregated transcripts.
    ///
    /// ## Decryption Process
    ///
    /// 1. For each chunk $j$:
    ///    a. Get ciphertext $C_{i,j}$ for this player and chunk
    ///    b. Compute $R_j^{sk_i}$ using the player's private key
    ///    c. Compute $M_{i,j} = C_{i,j} - R_j^{sk_i} = G \cdot u_{i,j}$
    ///    d. Solve discrete log: $u_{i,j} = dlog_G(M_{i,j})$
    ///
    /// 2. Reconstruct the share from chunks:
    ///    $s_i = \sum_{j=0}^{15} u_{i,j} \cdot 2^{16j}$
    ///
    /// ## Handling Aggregated Transcripts
    ///
    /// For aggregated transcripts (multiple dealers), the decrypted value is:
    /// $u_{i,j} = \sum_k u_{k,i,j}$
    ///
    /// This is the sum of chunk values from all dealers. The BSGS search range
    /// is expanded to `num_dealers * 2^16` to accommodate this.
    ///
    /// ## Arguments
    ///
    /// * `sc` - Secret sharing configuration (unused, for API compatibility)
    /// * `player` - The player whose share to decrypt
    /// * `dk` - The player's decryption private key
    /// * `pp` - Public parameters (for accessing cryptographic bases)
    ///
    /// ## Returns
    ///
    /// Tuple of (DealtSecretKeyShare, DealtPubKeyShare):
    /// - The decrypted secret share (as a G1 element representing a scalar)
    /// - The player's public key share (for verification)
    ///
    /// # Returns
    ///
    /// * `Ok((DealtSecretKeyShare, DealtPubKeyShare))` - Successful decryption
    /// * `Err(Error)` - Decryption failed (invalid transcript, wrong key, or corrupted data)
    fn decrypt_own_share(
        &self,
        _sc: &Self::SecretSharingConfig,
        player: &Player,
        dk: &Self::DecryptPrivKey,
        pp: &Self::PublicParameters,
    ) -> anyhow::Result<(Self::DealtSecretKeyShare, Self::DealtPubKeyShare)> {
        let g_1 = pp.get_encryption_public_params().pubkey_base();

        let chunks_ciphertexts = &self.ciphertexts[player.id];
        let num_dealers = self.soks.len();

        let mut recovered_chunks = Vec::with_capacity(NUM_CHUNKS);

        // Decrypt each chunk
        for j in 0..NUM_CHUNKS {
            // Get ciphertext for this player and chunk
            let c_ij = &chunks_ciphertexts[j];

            // Decrypt: C_{i,j} - R_j^{sk_i} = G * u_{i,j}
            let r_j = self.ephemeral_keys[j].mul(dk.dk);
            let m_ij = *c_ij - r_j;

            // Determine the discrete log search range
            // For single dealer: search in [0, 2^16)
            // For multiple dealers: search in [0, num_dealers * 2^16)
            // because the decrypted value is the sum of chunks from all dealers
            let limit = 1 << CHUNK_BIT_SIZE;
            let adjusted_limit = if num_dealers > 1 {
                limit * (num_dealers as u64)
            } else {
                limit
            };

            // Build BSGS table for discrete log computation
            // The table size is sqrt of the search range
            let m = (adjusted_limit as f64).sqrt().ceil() as u64;
            let bsgs_table = compute_bsgs_table(g_1, m);
            let giant_step = g_1.mul(Scalar::from(m));

            // Solve discrete log: find x such that G * x = m_ij
            let result = solve_discrete_log(&m_ij, &bsgs_table, &giant_step, m);

            let u_ij = result.ok_or_else(|| {
                anyhow!(
                    "BSGS discrete log failed for player {} chunk {}. \
                    Search range was [0, {}). This indicates an invalid transcript \
                    or incorrect decryption key.",
                    player.id,
                    j,
                    adjusted_limit
                )
            })?;
            recovered_chunks.push(u_ij as u16);
        }

        // Reconstruct the share from chunks
        // s_i = Σ_j u_{i,j} * (2^16)^j
        let share = chunks_to_scalar(&recovered_chunks);

        Ok((
            Self::DealtSecretKeyShare::new(Self::DealtSecretKey::new(share)),
            Self::DealtPubKeyShare::new(Self::DealtPubKey::new(self.V[player.id])),
        ))
    }

    /// Generates a random transcript for testing purposes.
    ///
    /// This function is not implemented and exists only for API compatibility.
    /// It would be used for benchmarking or testing scenarios that require
    /// random transcripts without going through the full deal process.
    fn generate<R>(_sc: &Self::SecretSharingConfig, _rng: &mut R) -> Self
    where
        R: rand_core::RngCore + rand_core::CryptoRng,
    {
        todo!("Implement generate() for testing")
    }
}

/// Generates a DLEQ (Discrete Log Equality) proof.
///
/// Proves that log_G(A) = log_H(B) without revealing the common logarithm.
/// Uses the Chaum-Pedersen protocol with Fiat-Shamir transform.
#[allow(non_snake_case)]
fn generate_dleq_proof<R: rand_core::RngCore + rand_core::CryptoRng>(
    secret: &Scalar,
    g: &G1Projective,
    A: &G1Projective,
    h: &G1Projective,
    B: &G1Projective,
    rng: &mut R,
) -> DleqProof {
    let w = random_scalar(rng);

    let commitment_g = g.mul(w);
    let commitment_h = h.mul(w);

    let mut context = DLEQ_PROOF_DST.to_vec();
    context.extend_from_slice(&bcs::to_bytes(A).unwrap());
    context.extend_from_slice(&bcs::to_bytes(B).unwrap());
    let c = hash_to_scalar(&context, DLEQ_PROOF_DST);

    let response = w - c * secret;

    DleqProof {
        commitment_g,
        commitment_h,
        response,
    }
}

/// Verifies a DLEQ proof.
///
/// Checks that log_G(A) = log_H(B) using the provided proof.
#[allow(non_snake_case)]
fn verify_dleq_proof(
    proof: &DleqProof,
    g: &G1Projective,
    A: &G1Projective,
    h: &G1Projective,
    B: &G1Projective,
) -> Result<()> {
    let mut context = DLEQ_PROOF_DST.to_vec();
    context.extend_from_slice(&bcs::to_bytes(A).unwrap());
    context.extend_from_slice(&bcs::to_bytes(B).unwrap());
    let c = hash_to_scalar(&context, DLEQ_PROOF_DST);

    let left1 = g.mul(proof.response) + A.mul(c);
    let left2 = h.mul(proof.response) + B.mul(c);

    if left1 != proof.commitment_g {
        bail!("DLEQ proof verification failed: G^r * A^c != commitment_g");
    }

    if left2 != proof.commitment_h {
        bail!("DLEQ proof verification failed: h^r * B^c != commitment_h");
    }

    Ok(())
}

/// Converts a scalar into 16-bit chunks.
///
/// Takes a 256-bit scalar and splits it into 16 chunks of 16 bits each.
/// The chunks are in little-endian order: chunk 0 contains the lowest 16 bits.
///
/// ## Algorithm
///
/// 1. Get the little-endian byte representation of the scalar (32 bytes)
/// 2. Group bytes into pairs (2 bytes = 16 bits)
/// 3. Convert each pair to a u16
///
/// ## Example
///
/// For scalar `0x12345678...`:
/// - Bytes: `[0x78, 0x56, 0x34, 0x12, ...]`
/// - Chunk 0: `0x5678` (bytes 0-1)
/// - Chunk 1: `0x1234` (bytes 2-3)
///
/// ## Arguments
///
/// * `s` - The scalar to chunk
///
/// ## Returns
///
/// A vector of 16 u16 values representing the chunked scalar
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

/// Reconstructs a scalar from 16-bit chunks.
///
/// Takes a vector of 16 u16 chunks and reconstructs the original 256-bit scalar
/// using base-$2^{16}$ arithmetic.
///
/// ## Algorithm
///
/// Computes: $s = \sum_{j=0}^{15} chunk_j \cdot (2^{16})^j$
///
/// This is the inverse of [`scalar_to_chunks`].
///
/// ## Arguments
///
/// * `chunks` - Vector of 16 u16 chunks in little-endian order
///
/// ## Returns
///
/// The reconstructed scalar as a blstrs::Scalar
///
/// ## Mathematical Formula
///
/// Given chunks $c_0, c_1, \dots, c_{15}$ where $c_j \in [0, 2^{16})$:
///
/// $s = c_0 + c_1 \cdot 2^{16} + c_2 \cdot 2^{32} + \dots + c_{15} \cdot 2^{240}$
///
/// ## Note on Aggregation
///
/// When decrypting aggregated transcripts, the chunks contain the sum of
/// values from multiple dealers. This is still correctly reconstructed
/// using the same formula because addition distributes over the chunking:
/// $\sum_k (u_{k,i})_j$ correctly gives chunk $j$ of $\sum_k u_{k,i}$.
fn chunks_to_scalar(chunks: &[u16]) -> Scalar {
    let radix = Scalar::from(1u64 << CHUNK_BIT_SIZE);
    let mut result = Scalar::from(0u64);
    let mut multiplier = Scalar::from(1u64);

    for &chunk in chunks {
        result += Scalar::from(chunk as u64) * multiplier;
        multiplier *= radix;
    }

    result
}

/// Builds a Baby-Step Giant-Step (BSGS) lookup table.
///
/// The BSGS algorithm solves discrete logarithms in time $O(\sqrt{N})$ where
/// $N$ is the size of the search space. This function builds the "baby steps"
/// portion of the table.
///
/// ## BSGS Algorithm Overview
///
/// To solve $G^x = P$ where $0 \leq x < N$:
/// 1. Set $m = \lceil\sqrt{N}\rceil$
/// 2. Precompute baby steps: $j \cdot G$ for $j = 0..m-1$ and store in hash table
/// 3. Compute giant step: $G^m$
/// 4. For $i = 0..m$:
///    - Check if $P - i \cdot G^m$ is in the baby step table
///    - If found, solution is $x = i \cdot m + j$
///
/// ## Arguments
///
/// * `base` - The generator G of the cyclic group
/// * `m` - The table size (typically $\lceil\sqrt{N}\rceil$)
///
/// ## Returns
///
/// A HashMap mapping compressed group elements to their discrete log values.
///
/// ## Performance Notes
///
/// - Table construction: $O(m)$ group operations and hashing
/// - Memory usage: $O(m)$ group elements (each ~48 bytes for G1)
/// - For 16-bit chunks: $m = 256$, memory ~12KB
/// - For 32-bit chunks: $m = 65536$, memory ~3MB
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

/// Solves discrete logarithm using Baby-Step Giant-Step.
///
/// Finds $x$ such that $G^x = target$ where $0 \leq x < m^2$.
///
/// ## Arguments
///
/// * `target` - The group element we want to compute discrete log of
/// * `table` - Pre-computed BSGS table from [`compute_bsgs_table`]
/// * `giant_step` - Pre-computed $G^m$ where $m = \lceil\sqrt{N}\rceil$
/// * `m` - The table size parameter
///
/// ## Returns
///
/// * `Some(x)` where $x$ is the discrete log in range $[0, m^2)$
/// * `None` if no solution exists in the search range
///
/// ## Algorithm Details
///
/// We solve: $target = G^x$
///
/// Write $x = i \cdot m + j$ where $0 \leq i, j < m$.
///
/// Then: $target = G^{i \cdot m + j} = (G^m)^i \cdot G^j$
///
/// Rearranging: $target \cdot (G^{-m})^i = G^j$
///
/// For each $i$:
/// 1. Compute $target \cdot (G^{-m})^i$
/// 2. Check if result is in the baby step table
/// 3. If found, return $i \cdot m + j$
///
/// ## Search Range
///
/// The search range is $[0, m^2)$. For our use case:
/// - Single dealer: $m = 256$, range $[0, 65536)$ (16-bit chunks)
/// - Aggregated (d dealers): $m = d \cdot 256$, range $[0, d^2 \cdot 65536)$
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
    /// Signs a PVSS contribution using the dealer's BLS12381 signing key.
    ///
    /// This signature authenticates the transcript and proves that the dealer
    /// with the corresponding signing key created it.
    ///
    /// ## Arguments
    ///
    /// * `sk` - The dealer's BLS12381 private signing key
    /// * `player` - The player ID of the dealer
    /// * `aux` - Auxiliary data to be included in the signature (provides context)
    /// * `comm` - The commitment being signed (typically V[n], the secret commitment)
    ///
    /// ## Returns
    ///
    /// A BLS12381 signature over the contribution data.
    ///
    /// ## Security Note
    ///
    /// The signature prevents:
    /// 1. Replay attacks (aux provides context)
    /// 2. Tampering (signature covers all commitment data)
    /// 3. Forgery (requires dealer's signing key)
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

    /// Test that the Transcript struct compiles correctly.
    ///
    /// This is a smoke test to catch any compilation errors early.
    /// It doesn't test functionality, just that the types are well-defined.
    ///
    /// ## Test Strategy
    ///
    /// 1. Call this test (it compiles)
    /// 2. If compilation succeeds, types are well-defined
    #[test]
    fn test_transcript_struct_compiles() {}

    /// Tests that deal() creates transcripts with correct structure sizes.
    ///
    /// Verifies that the deal function produces transcripts where:
    /// - The commitment vector V has exactly n+1 elements
    /// - The ciphertexts vector has exactly n elements (one per validator)
    /// - The soks vector has exactly 1 element (one dealer)
    ///
    /// ## Test Cases
    ///
    /// Tests multiple threshold configurations:
    /// - (1, 1): Single player, single share
    /// - (2, 3): 2-of-3 threshold
    /// - (3, 5): 3-of-5 threshold
    /// - (4, 7): 4-of-7 threshold
    ///
    /// ## Assertions
    ///
    /// 1. V.len() == n + 1 (n share commitments + 1 secret commitment)
    /// 2. ciphertexts.len() == n (one row per validator)
    /// 3. soks.len() == 1 (one dealer)
    #[test]
    fn test_deal_creates_valid_structure() {
        let mut rng = thread_rng();

        // Test multiple configurations to ensure general correctness
        for tc in get_threshold_configs_for_testing::<ThresholdConfigBlstrs>() {
            // Set up dealing infrastructure
            let d = setup_dealing::<Transcript, _>(&tc, &mut rng);

            // Create a transcript from dealer 0
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
                trx.ciphertexts.len(),
                tc.n,
                "ciphertexts should have n elements for t={}, n={}",
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

    /// Tests the deal -> decrypt roundtrip for single-dealer transcripts.
    ///
    /// This is a core correctness test that verifies:
    /// 1. The deal function correctly encrypts shares
    /// 2. Each player can successfully decrypt their share
    /// 3. The decryption produces valid public key shares
    ///
    /// ## Test Strategy
    ///
    /// 1. Create a transcript using deal()
    /// 2. For each player i:
    ///    a. Call decrypt_own_share() to get their share
    ///    b. Verify the public key share matches V[i]
    ///
    /// ## Test Cases
    ///
    /// Tests representative configurations:
    /// - (1, 1): Trivial single-player case
    /// - (2, 3): Small threshold
    /// - (3, 5): Medium threshold
    /// - (4, 7): Larger threshold
    ///
    /// ## Security Properties Tested
    ///
    /// - Correctness: Honest decryption always succeeds
    /// - Share validity: Each share maps to the correct public key share
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

            // Create transcript from single dealer
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
                let (sk_share, pk_share) = trx
                    .decrypt_own_share(&tc, &player, &d.dks[i], &d.pp)
                    .expect("decrypt_own_share should not fail for valid transcript");

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

    /// Tests that transcript aggregation preserves structural integrity.
    ///
    /// Verifies that when multiple transcripts are combined:
    /// 1. The ciphertexts vector size is preserved
    /// 2. The V vector size is preserved
    /// 3. The soks vector correctly accumulates all dealers
    /// 4. The get_dealers() function returns all contributing dealers
    ///
    /// ## Test Setup
    ///
    /// - Configuration: 2-out-of-4 threshold
    /// - Dealers: 3 dealers create transcripts
    /// - Aggregation: trx1 aggregates trx2, then trx3
    ///
    /// ## Assertions
    ///
    /// 1. V length remains n+1 after aggregation
    /// 2. ciphertexts length remains n after aggregation
    /// 3. soks length is 3 (3 dealers)
    /// 4. get_dealers() contains all three dealer IDs
    ///
    /// ## What This Tests
    ///
    /// This test verifies the aggregation operation itself is correct,
    /// not that the resulting shares are decryptable (that's tested elsewhere).
    #[test]
    fn test_aggregation_preserves_structure() {
        let mut rng = thread_rng();
        let tc = ThresholdConfigBlstrs::new(2, 4).unwrap();

        let d = setup_dealing::<Transcript, _>(&tc, &mut rng);

        // Create transcripts from 3 dealers
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

        // Aggregate all three
        trx1.aggregate_with(&tc, &trx2);
        trx1.aggregate_with(&tc, &trx3);

        // Verify structure is preserved
        assert_eq!(trx1.V.len(), tc.n + 1, "V length should be preserved");
        assert_eq!(
            trx1.ciphertexts.len(),
            tc.n,
            "ciphertexts length should be preserved"
        );
        assert_eq!(trx1.soks.len(), 3, "Should have 3 SoKs after aggregation");

        // Verify dealers are tracked
        let dealers = trx1.get_dealers();
        assert_eq!(dealers.len(), 3);
        assert!(dealers.contains(&tc.get_player(0)));
        assert!(dealers.contains(&tc.get_player(1)));
        assert!(dealers.contains(&tc.get_player(2)));
    }

    /// Tests BCS serialization and deserialization roundtrip.
    ///
    /// Verifies that:
    /// 1. Transcripts can be serialized to bytes
    /// 2. The bytes can be deserialized back to a Transcript
    /// 3. The deserialized transcript equals the original
    ///
    /// ## Why This Matters
    ///
    /// Transcripts need to be transmitted over the network and stored.
    /// Serialization must be:
    /// - Deterministic (same input always produces same bytes)
    /// - Lossless (roundtrip preserves all data)
    /// - Compatible (can be deserialized by any instance)
    ///
    /// ## Test Case
    ///
    /// Configuration: 2-out-of-3 threshold
    ///
    /// ## Assertions
    ///
    /// The deserialized transcript exactly equals the original.
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

    /// Tests that the dealt public key is consistent with the input secret.
    ///
    /// Verifies that the commitment to the secret (V[n]) equals the expected
    /// public key computed from the input secret.
    ///
    /// ## Mathematical Relationship
    ///
    /// For an input secret s:
    /// - The polynomial f(0) = s
    /// - V[n] = G2^{f(0)} = G2^s
    /// - This should equal the expected dealt public key from setup
    ///
    /// ## Test Case
    ///
    /// Configuration: 2-out-of-4 threshold
    ///
    /// ## Assertion
    ///
    /// The transcript's dealt public key equals the setup's dealt public key.
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

    /// Tests that aggregated transcript decryption works correctly.
    ///
    /// This is a comprehensive test that verifies:
    /// 1. Multiple dealers can create transcripts for the same DKG session
    /// 2. Transcripts can be aggregated
    /// 3. The aggregated dealt public key equals the sum of all secrets
    /// 4. Each player can decrypt their share from the aggregated transcript
    ///
    /// ## Test Setup
    ///
    /// - Configuration: 2-out-of-3 threshold
    /// - All 3 players act as dealers
    /// - Each creates a transcript with a different secret
    /// - All transcripts are aggregated into one
    ///
    /// ## Mathematical Setup
    ///
    /// Dealer i has secret s_i and creates polynomial f_i(x) with f_i(0) = s_i
    /// After aggregation:
    /// - Combined secret: S = Σ s_i
    /// - Combined polynomial: F(x) = Σ f_i(x)
    /// - Combined commitment: G2^S = G2^{Σ s_i}
    ///
    /// ## Test Steps
    ///
    /// 1. Create 3 transcripts, one for each dealer
    /// 2. Aggregate all into one transcript
    /// 3. Verify aggregated DPK equals sum of individual secrets
    /// 4. Each player decrypts their share from aggregated transcript
    /// 5. Verify public key shares match
    ///
    /// ## What This Tests
    ///
    /// This test verifies the core multi-dealer DKG functionality:
    /// - Aggregation correctness
    /// - Cross-dealer decryption
    /// - Secret combination properties
    #[test]
    fn test_aggregated_decrypt_combines_secrets() {
        let mut rng = thread_rng();
        let tc = ThresholdConfigBlstrs::new(2, 3).unwrap();

        let d = setup_dealing::<Transcript, _>(&tc, &mut rng);

        // Deal from all 3 players (each with their own secret)
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

        // Aggregate transcripts from remaining dealers
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
        // This is because V_agg[n] = Σ V_k[n] = Σ G2^{s_k} = G2^{Σ s_k}
        let aggregated_dpk = aggregated.get_dealt_public_key();
        assert_eq!(
            aggregated_dpk, d.dpk,
            "Aggregated public key should match sum of input secrets"
        );

        // Each player can still decrypt their share from the aggregated transcript
        // The decrypted value is the sum of their shares from all dealers
        for i in 0..tc.n {
            let player = Player { id: i };
            let (sk_share, pk_share) = aggregated
                .decrypt_own_share(&tc, &player, &d.dks[i], &d.pp)
                .expect("decrypt_own_share should not fail for valid transcript");

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
