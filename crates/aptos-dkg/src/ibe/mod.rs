// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Identity-Based Encryption (IBE) primitives for timelock encryption.
//!
//! This module implements a Boneh-Franklin style IBE scheme using BLS12-381:
//!
//! - **Master Public Key (MPK)**: A G2 element `mpk = g2^s` where `s` is the master secret
//! - **Identity**: A domain-specific string hashed to G1
//! - **Decryption Key**: For identity `id`, the key is `dk = H(id)^s` in G1
//! - **Encryption**: Uses the pairing `e(H(id), mpk)` to derive a symmetric key
//! - **Decryption**: Uses the pairing `e(dk, U)` to recover the symmetric key
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                         IBE Encryption Flow                              │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                          │
//! │  Input: (mpk, identity, message)                                        │
//! │         │                                                               │
//! │         ▼                                                               │
//! │  1. h = H(identity) ──► G1 point                                        │
//! │  2. r ← random scalar                                                   │
//! │  3. U = g2^r ──► G2 element (sent with ciphertext)                      │
//! │  4. g_id = e(h, mpk)^r ──► Gt element                                   │
//! │  5. k = SHA3-256(g_id)[0:32] ──► symmetric key                          │
//! │  6. V = message XOR k ──► ciphertext payload                            │
//! │                                                                          │
//! │  Output: (U, V) ciphertext                                               │
//! └─────────────────────────────────────────────────────────────────────────┘
//!
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                         IBE Decryption Flow                              │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                          │
//! │  Input: (dk, identity, ciphertext = (U, V))                             │
//! │         │                                                               │
//! │         ▼                                                               │
//! │  1. h = H(identity) ──► G1 point                                        │
//! │  2. g_id = e(dk, U) ──► Gt element (reconstructs same as encrypt)       │
//! │  3. k = SHA3-256(g_id)[0:32] ──► symmetric key                          │
//! │  4. message = V XOR k ──► plaintext                                     │
//! │                                                                          │
//! │  Output: message                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Integration with DKG
//!
//! The IBE module consumes the output of the Scalar ElGamal PVSS:
//! - DKG produces scalar shares via [`scalar_elgamal::Transcript`](../../pvss/scalar_elgamal/transcript.rs)
//! - Shares are reconstructed to get the master secret `s`
//! - Decryption keys are derived: `dk = H(id)^s`
//!
//! See [`transcript.rs`](../../pvss/scalar_elgamal/transcript.rs) for DKG integration.
//!
//! ## Documentation References
//!
//! **Core Documentation:**
//! - [ADR-001: Dual-Output DKG](atomica/docs/adr-001-dual-output-dkg.md)
//! - [Implementation Plan](atomica/docs/implementation-plan-unified-dkg-ibe.md)
//! - [Definitions](atomica/docs/definitions.md)
//!
//! **Technical Details:**
//! - [Chunked ElGamal Scalar Generation](atomica/docs/technical/chunked-elgamal-scalar-generation.md)
//! - [Scalar ElGamal PVSS](../pvss/scalar_elgamal/transcript.rs)
//!
//! **Testing:**
//! - [IBE Unit Tests](tests.rs)
//! - [mpk_encrypt_decrypt Smoke Test](../../../../testsuite/smoke-test/src/timelock/mpk_encrypt_decrypt.rs)
//!
//! ## Security
//!
//! The IBE scheme relies on the Bilinear Diffie-Hellman (BDH) assumption.
//! The identity derivation includes the timelock ID and deadline to prevent
//! cross-timelock attacks.
//!
//! ## File Structure
//!
//! - [`mod.rs`](mod.rs) - Core IBE primitives (encrypt, decrypt, key derivation)
//! - [`ciphertext.rs`](ciphertext.rs) - Ciphertext structure and serialization
//! - [`tests.rs`](tests.rs) - Unit tests with known scalar examples
//!

pub mod ciphertext;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod golden_vectors;

pub use ciphertext::Ciphertext;

use crate::pvss::dealt_secret_key::scalar::DealtSecretKey;
use crate::utils::random::random_scalar_from_uniform_bytes;
use aptos_crypto::blstrs::SCALAR_NUM_BYTES;
use blstrs::{pairing, G1Affine, G1Projective, G2Affine, G2Projective, Scalar};
use group::{Curve, Group};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::fs;
use std::ops::Mul;
use std::path::Path;

/// Errors that can occur during IBE operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IbeError {
    /// Validator indices and shares have different lengths
    ValidatorIndicesSharesMismatch {
        indices_len: usize,
        shares_len: usize,
    },
    /// No validators participating in reconstruction
    EmptyShares,
    /// Total weight doesn't match sum of individual weights
    WeightSumMismatch {
        total_weight: u64,
        computed_sum: u64,
    },
    /// Number of shares doesn't match validator's weight
    ShareWeightMismatch {
        validator_index: u64,
        shares_count: usize,
        expected_weight: u64,
    },
    /// Failed to create weighted configuration
    InvalidWeightedConfig(String),
    /// Invalid G1 share format (not 48 bytes or invalid point)
    InvalidG1Share(String),
}

impl std::fmt::Display for IbeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IbeError::ValidatorIndicesSharesMismatch {
                indices_len,
                shares_len,
            } => {
                write!(
                    f,
                    "validator_indices ({}) and scalar_shares ({}) must have same length",
                    indices_len, shares_len
                )
            },
            IbeError::EmptyShares => write!(f, "Cannot reconstruct DK from empty shares"),
            IbeError::WeightSumMismatch {
                total_weight,
                computed_sum,
            } => write!(
                f,
                "total_weight ({}) must equal sum of weights ({})",
                total_weight, computed_sum
            ),
            IbeError::ShareWeightMismatch {
                validator_index,
                shares_count,
                expected_weight,
            } => write!(
                f,
                "Validator {} has {} shares but weight is {}",
                validator_index, shares_count, expected_weight
            ),
            IbeError::InvalidWeightedConfig(msg) => write!(f, "Invalid weighted config: {}", msg),
            IbeError::InvalidG1Share(msg) => write!(f, "Invalid G1 share: {}", msg),
        }
    }
}

impl std::error::Error for IbeError {}

/// Domain separation tag for IBE identity hashing.
/// Ensures unique hash domains for IBE vs other protocols.
pub const IBE_IDENTITY_DST: &[u8] = b"APTOS_IBE_IDENTITY_DST";

/// Domain separation tag for symmetric key derivation from pairing result.
/// Prevents key confusion between IBE and other uses of the pairing output.
pub const IBE_KEY_DERIVATION_DST: &[u8] = b"APTOS_IBE_KEY_DERIVATION_DST";

/// IBE roundtrip test vector with DK share reconstruction
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IbeRoundtripVector {
    pub description: String,
    pub rng_seed: u64,
    pub threshold: u64,
    pub total_weight: u64,
    pub msk_hex: String,
    pub mpk_g2_hex: String,
    pub identity_hash_hex: String,
    pub h_identity_g1_hex: String,
    pub validator_indices: Vec<u64>,
    pub validator_weights: Vec<u64>,
    /// Nested vector of DK shares: vector<vector<G1_compressed_hex>>
    /// Outer index: validator, inner index: virtual player share
    pub dk_shares_g1_hex: Vec<Vec<String>>,
    pub reconstructed_dk_g1_hex: String,
    pub plaintext_hex: String,
    pub ciphertext_u_g2_hex: String,
    pub ciphertext_v_hex: String,
}

/// Golden vectors container
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GoldenVectors {
    pub ibe_roundtrip_vectors: Vec<IbeRoundtripVector>,
}

/// Load IBE golden vectors from the standard location.
///
/// The golden vectors are used across:
/// - Rust unit tests
/// - Move VM native function tests
/// - Move language unit tests
///
/// Returns `None` if the file cannot be loaded.
pub fn load_golden_vectors() -> Option<GoldenVectors> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").ok()?;
    let workspace_root = Path::new(&manifest_dir).parent()?.parent()?;
    let golden_vectors_path = workspace_root.join("atomica/golden_vectors/ibe_golden_vectors.json");

    let contents = fs::read_to_string(&golden_vectors_path).ok()?;
    serde_json::from_str(&contents).ok()
}

/// Computes an IBE identity from a timelock ID and deadline.
///
/// The identity is a 32-byte hash that uniquely identifies a timelock
/// for encryption purposes. The identity is computed as:
///
/// ```
/// identity = SHA3-256(IBE_IDENTITY_DST || timelock_id || deadline_us)
/// ```
///
/// This construction ensures:
/// 1. **Uniqueness**: Each (timelock_id, deadline_us) pair maps to a unique identity
/// 2. **Domain Separation**: Different protocols use different DSTs
/// 3. **Collision Resistance**: SHA3-256 provides 128-bit security
///
/// # Arguments
/// * `timelock_id` - Unique identifier for the timelock (assigned on registration)
/// * `deadline_us` - Deadline in microseconds since epoch (when decryption becomes possible)
///
/// # Returns
/// A 32-byte identity hash suitable for IBE encryption
///
/// # Example
///
/// ```
/// let identity = compute_identity(1, 1704067200000000); // Deadline: 2024-01-01
/// assert_eq!(identity.len(), 32);
/// ```
///
/// # See Also
///
/// - [`hash_to_g1()`] - Maps identity to G1 curve point
/// - [`derive_decryption_key()`] - Derives decryption key from identity and secret
/// - [Timelock Registration](atomica/docs/technical/timelock-registration.md)
pub fn compute_identity(timelock_id: u64, deadline_us: u64) -> [u8; 32] {
    let mut hasher = Sha3_256::new();
    hasher.update(IBE_IDENTITY_DST);
    hasher.update(timelock_id.to_le_bytes());
    hasher.update(deadline_us.to_le_bytes());
    hasher.finalize().into()
}

/// Hashes an identity to a G1 curve point.
///
/// Uses the standard hash-to-curve construction for BLS12-381 G1.
/// The result is used in the pairing operations for encryption/decryption.
///
/// # Arguments
/// * `identity` - The 32-byte identity bytes (from [`compute_identity()`])
///
/// # Returns
/// A G1 projective point representing the hashed identity
///
/// # Algorithm
///
/// Uses the `hash_to_curve` method from `blstrs`:
/// ```text
/// Q_id = H*(identity || domain_separation_tag)
/// ```
/// where H* is the random oracle construction for BLS12-381 G1.
///
/// # See Also
///
/// - [`compute_identity()`] - Creates the identity bytes
/// - [`derive_decryption_key()`] - Uses the G1 point to derive decryption keys
/// - [Boneh-Franklin IBE](atomica/docs/definitions.md#boneh-franklin-identity-based-encryption-ibe)
pub fn hash_to_g1(identity: &[u8]) -> G1Projective {
    G1Projective::hash_to_curve(identity, IBE_IDENTITY_DST, b"H(id)")
}

/// Derives the decryption key for an identity given the master secret.
///
/// Computes `dk = H(identity)^secret` where H maps the identity to G1.
/// This is the "Extract" algorithm in Boneh-Franklin IBE.
///
/// # Arguments
/// * `secret` - The master secret scalar (from DKG) or a threshold share
/// * `identity` - The identity bytes (from [`compute_identity()`])
///
/// # Returns
/// The decryption key as a G1 affine point
///
/// # Example
///
/// ```
/// let secret = Scalar::from(42u64);
/// let identity = compute_identity(1, 1704067200000000);
/// let dk = derive_decryption_key(&secret, &identity);
/// ```
///
/// # Security Notes
///
/// - The secret should come from DKG reconstruction or be a threshold share
/// - For threshold decryption, each validator computes their contribution:
///   `dk_i = H(identity)^share_i`
/// - Contributions are combined with Lagrange coefficients to get full DK
///
/// # See Also
///
/// - [`compute_identity()`] - Creates identity from timelock parameters
/// - [`hash_to_g1()`] - Maps identity to G1
/// - [DKG Integration](atomica/docs/implementation-plan-unified-dkg-ibe.md)
pub fn derive_decryption_key(secret: &Scalar, identity: &[u8]) -> G1Affine {
    let h = hash_to_g1(identity);
    h.mul(secret).to_affine()
}

/// Encrypts a message using IBE.
///
/// Given the master public key and an identity, encrypts the message so that
/// only the holder of the corresponding decryption key can decrypt it.
///
/// # Encryption Process
/// 1. Hash identity to G1: `h = H(identity)`
/// 2. Sample random scalar `r`
/// 3. Compute `U = g2^r`
/// 4. Compute symmetric key `k = H'(e(h, mpk)^r)`
/// 5. XOR message with key stream derived from k
///
/// # Arguments
/// * `mpk` - The master public key (G2 element)
/// * `identity` - The identity bytes
/// * `msg` - The plaintext message
/// * `rng` - Random number generator
///
/// # Returns
/// The ciphertext containing U and the encrypted message
pub fn ibe_encrypt<R: rand::Rng>(
    mpk: &G2Affine,
    identity: &[u8],
    msg: &[u8],
    rng: &mut R,
) -> Ciphertext {
    // Hash identity to G1
    let h = hash_to_g1(identity);

    // Sample random scalar r
    let r = random_scalar(rng);

    // Compute U = g2^r
    let u = G2Projective::generator().mul(&r);

    // Compute pairing e(h, mpk)^r = e(h^r, mpk) = e(h, mpk^r)
    // We use e(h^r, mpk) for efficiency
    let h_r = h.mul(&r).to_affine();
    let pairing_result = pairing(&h_r, mpk);

    // Derive symmetric key from pairing result
    let key_stream = derive_key_stream(&pairing_result, msg.len());

    // XOR message with key stream
    let v: Vec<u8> = msg
        .iter()
        .zip(key_stream.iter())
        .map(|(m, k)| m ^ k)
        .collect();

    Ciphertext::new(u, v)
}

/// Decrypts a ciphertext using the decryption key.
///
/// # Decryption Process
/// 1. Compute `e(dk, U)` where dk is the decryption key
/// 2. Derive symmetric key `k = H'(e(dk, U))`
/// 3. XOR ciphertext with key stream to recover message
///
/// Note: `e(dk, U) = e(H(id)^s, g2^r) = e(H(id), g2)^(sr) = e(H(id), mpk)^r`
///
/// # Arguments
/// * `dk` - The decryption key for the identity
/// * `ciphertext` - The ciphertext to decrypt
///
/// # Returns
/// The decrypted plaintext message
pub fn ibe_decrypt(dk: &G1Affine, ciphertext: &Ciphertext) -> Vec<u8> {
    // Compute pairing e(dk, U)
    let pairing_result = pairing(dk, &ciphertext.u);

    // Derive symmetric key from pairing result
    let key_stream = derive_key_stream(&pairing_result, ciphertext.payload_len());

    // XOR ciphertext with key stream to recover plaintext
    ciphertext
        .payload()
        .iter()
        .zip(key_stream.iter())
        .map(|(c, k)| c ^ k)
        .collect()
}

/// Derives a key stream from a pairing result (Gt element).
///
/// Uses SHA3-256 in counter mode to generate arbitrary-length key material.
fn derive_key_stream(gt: &blstrs::Gt, len: usize) -> Vec<u8> {
    let mut key_stream = Vec::with_capacity(len);
    let mut counter: u32 = 0;

    // Serialize Gt element for hashing
    let gt_bytes = gt_to_bytes(gt);

    while key_stream.len() < len {
        let mut hasher = Sha3_256::new();
        hasher.update(IBE_KEY_DERIVATION_DST);
        hasher.update(&gt_bytes);
        hasher.update(counter.to_le_bytes());
        let block = hasher.finalize();
        key_stream.extend_from_slice(&block);
        counter += 1;
    }

    key_stream.truncate(len);
    key_stream
}

/// Serializes a Gt element to bytes for hashing.
///
/// Uses BCS serialization which provides a deterministic byte representation.
fn gt_to_bytes(gt: &blstrs::Gt) -> Vec<u8> {
    // Gt implements serde::Serialize, so we can use BCS
    bcs::to_bytes(gt).expect("Gt serialization should never fail")
}

/// Generates a random scalar using the provided RNG.
///
/// Uses the same approach as the rest of the codebase to work around
/// rand_core version incompatibilities.
fn random_scalar<R: rand::Rng>(rng: &mut R) -> Scalar {
    // Generate 64 random bytes (2 * SCALAR_NUM_BYTES) and reduce modulo the field order
    let mut bytes = [0u8; 2 * SCALAR_NUM_BYTES];
    rng.fill(&mut bytes);
    random_scalar_from_uniform_bytes(&bytes)
}

/// Verifies that a decryption key is valid for the given identity and MPK.
///
/// This is useful for validating decryption keys received from validators.
///
/// # Verification
/// Checks that `e(dk, g2) = e(H(identity), mpk)`
///
/// # Arguments
/// * `dk` - The decryption key to verify
/// * `identity` - The identity bytes
/// * `mpk` - The master public key
///
/// # Returns
/// `true` if the decryption key is valid, `false` otherwise
pub fn verify_decryption_key(dk: &G1Affine, identity: &[u8], mpk: &G2Affine) -> bool {
    let h = hash_to_g1(identity).to_affine();
    let g2 = G2Projective::generator().to_affine();

    // Check e(dk, g2) = e(h, mpk)
    let lhs = pairing(dk, &g2);
    let rhs = pairing(&h, mpk);

    lhs == rhs
}

// ============================================================================
// REFERENCE IMPLEMENTATION: Scalar-Based DK Reconstruction
// ============================================================================
// The following is a reference implementation for understanding the cryptographic
// flow. It is NOT used in production. The production system uses G1-based
// reconstruction via `reconstruct_ibe_dk_from_g1_shares()`.
//
// Production Flow:
// 1. Validators compute: dk_share_i = s_i * H(identity) (G1 point, 48 bytes)
// 2. Validators submit G1 points as TimelockShare transactions
// 3. On-chain: weighted Lagrange interpolation on G1 points
// 4. Result: DK = Σ(λ_j * dk_share_j)
//
// Reference Flow (NOT USED):
// 1. Validators submit scalar shares s_i
// 2. Reconstruct master secret s via Lagrange interpolation
// 3. Compute DK = H(identity)^s
//
// The reference implementation below shows how scalar reconstruction would work.
// It is kept for documentation and verification purposes only.
// ============================================================================

#[doc(hidden)]
#[deprecated(
    since = "0.1.0",
    note = "Reference only. Use reconstruct_ibe_dk_from_g1_shares() for production."
)]
pub fn _reconstruct_ibe_dk_reference(
    validator_indices: &[u64],
    scalar_shares: &[Vec<Scalar>],
    weights: &[u64],
    total_weight: u64,
    identity: &[u8; 32],
) -> Result<G1Affine, IbeError> {
    use crate::pvss::dealt_secret_key_share::scalar::DealtSecretKeyShare;
    use crate::pvss::scalar_elgamal::WeightedTranscript;
    use crate::pvss::traits::{Reconstructable, Transcript as TranscriptTrait};
    use crate::pvss::{Player, WeightedConfig};

    // Validate inputs
    if validator_indices.len() != scalar_shares.len() {
        return Err(IbeError::ValidatorIndicesSharesMismatch {
            indices_len: validator_indices.len(),
            shares_len: scalar_shares.len(),
        });
    }
    if validator_indices.is_empty() {
        return Err(IbeError::EmptyShares);
    }
    let computed_total: u64 = weights.iter().copied().sum();
    if total_weight != computed_total {
        return Err(IbeError::WeightSumMismatch {
            total_weight,
            computed_sum: computed_total,
        });
    }

    let weights_usize: Vec<usize> = weights.iter().map(|w| *w as usize).collect();
    let wconfig = WeightedConfig::new(validator_indices.len(), weights_usize)
        .map_err(|e| IbeError::InvalidWeightedConfig(e.to_string()))?;

    // Build share vector for reconstruction
    let mut shares_for_recon: Vec<(Player, Vec<DealtSecretKeyShare>)> =
        Vec::with_capacity(validator_indices.len());

    for (vi, &validator_idx) in validator_indices.iter().enumerate() {
        let weight: u64 = weights[validator_idx as usize];
        let shares: &[Scalar] = &scalar_shares[vi];

        if shares.len() != weight as usize {
            return Err(IbeError::ShareWeightMismatch {
                validator_index: validator_idx,
                shares_count: shares.len(),
                expected_weight: weight,
            });
        }

        let mut dealt_shares: Vec<DealtSecretKeyShare> = Vec::with_capacity(shares.len());
        for scalar_share in shares.iter() {
            dealt_shares.push(DealtSecretKeyShare::new(DealtSecretKey::new(*scalar_share)));
        }

        shares_for_recon.push((
            Player {
                id: validator_idx as usize,
            },
            dealt_shares,
        ));
    }

    // Reconstruct master secret
    let reconstructed_secret: DealtSecretKey =
        <WeightedTranscript as TranscriptTrait>::DealtSecretKey::reconstruct(
            &wconfig,
            &shares_for_recon,
        );

    // Derive DK: DK = H(identity)^s
    Ok(derive_decryption_key(&reconstructed_secret.s, identity))
}

/// Reconstructs an IBE decryption key from G1 DK shares.
///
/// This is the **production path** for IBE DK reconstruction in the timelock system.
/// Validators submit pre-computed G1 decryption key shares (48-byte compressed points),
/// and this function performs weighted Lagrange interpolation directly on G1 points.
///
/// # Production Flow
///
/// ```text
/// 1. Off-chain (Validator):
///    - Has scalar share s_i from DKG
///    - Computes identity = SHA3-256(timelock_id || deadline_us)
///    - Computes H(identity) = hash_to_g1(identity)
///    - Computes dk_share_i = s_i * H(identity)  → G1 point
///    - Submits 48-byte compressed dk_share_i as TimelockShare transaction
///
/// 2. On-chain (Move VM native):
///    - Collects G1 shares from submitting validators
///    - Computes weighted Lagrange coefficients λ_j(0)
///    - Computes DK = Σ(λ_j * dk_share_j)  → G1 point
///    - Stores DK on-chain for timelock
/// ```
///
/// # Arguments
///
/// * `virtual_player_ids` - Global indices of the participating virtual players (0 to total_weight-1)
/// * `dk_shares` - G1 DK shares (48-byte compressed), one per participating virtual player
/// * `total_weight` - Sum of all validator weights (determines the Lagrange evaluation domain)
///
/// # Returns
///
/// The reconstructed decryption key as a G1 affine point (48 bytes when compressed).
///
/// # Errors
///
/// * `ValidatorIndicesSharesMismatch` - If IDs and shares have different lengths
/// * `EmptyShares` - If no shares provided
/// * `InvalidG1Share` - If any share is not 48 bytes or is not a valid G1 point
pub fn reconstruct_ibe_dk_from_g1_shares(
    virtual_player_ids: &[u64],
    dk_shares: &[Vec<u8>],
    total_weight: u64,
) -> Result<G1Affine, IbeError> {
    use crate::algebra::lagrange::lagrange_coefficients;

    // ==========================================================================
    // Input Validation
    // ==========================================================================

    if virtual_player_ids.len() != dk_shares.len() {
        return Err(IbeError::ValidatorIndicesSharesMismatch {
            indices_len: virtual_player_ids.len(),
            shares_len: dk_shares.len(),
        });
    }

    if virtual_player_ids.is_empty() {
        return Err(IbeError::EmptyShares);
    }

    // ==========================================================================
    // Lagrange Coefficients
    // ==========================================================================
    // The evaluation domain is based on total_weight.
    let batch_dom =
        crate::algebra::evaluation_domain::BatchEvaluationDomain::new(total_weight as usize);

    let player_ids: Vec<usize> = virtual_player_ids.iter().map(|&idx| idx as usize).collect();
    let lagrange_coeffs = lagrange_coefficients(&batch_dom, &player_ids, &Scalar::from(0u64));

    // ==========================================================================
    // G1 Reconstruction
    // ==========================================================================
    let mut reconstructed = G1Projective::identity();

    for (i, dk_share_bytes) in dk_shares.iter().enumerate() {
        // Validate share format
        if dk_share_bytes.len() != 48 {
            return Err(IbeError::InvalidG1Share(format!(
                "DK share {} has {} bytes, expected 48",
                i,
                dk_share_bytes.len()
            )));
        }

        // Deserialize compressed G1
        let g1_bytes: &[u8; 48] = dk_share_bytes
            .as_slice()
            .try_into()
            .expect("DK share should be exactly 48 bytes");
        let g1_affine = G1Affine::from_compressed(g1_bytes)
            .expect(&format!("DK share {} is not a valid G1 point", i));

        // Accumulate: reconstructed += λ_i(0) × dk_share_i
        reconstructed += G1Projective::from(g1_affine) * lagrange_coeffs[i];
    }

    Ok(reconstructed.to_affine())
}

/// Test helper function for IBE DK reconstruction from secret shares.
///
/// This function is used in tests to verify DK reconstruction. It takes scalar
/// secret shares (not G1 DK shares) and reconstructs the decryption key by:
/// 1. Converting scalar shares to G1 DK shares using the identity
/// 2. Calling `reconstruct_ibe_dk_from_g1_shares` for the actual reconstruction
///
/// **Note:** This is for testing only. Production code should use
/// `reconstruct_ibe_dk_from_g1_shares()` directly with pre-computed G1 DK shares.
///
/// # Arguments
///
/// * `validator_indices` - Indices of participating validators (0-based)
/// * `scalar_shares` - Nested vector of scalar secret shares per validator
/// * `weights` - Weights for all validators (not just participating)
/// * `total_weight` - Sum of all validator weights
/// * `identity` - 32-byte identity hash for DK derivation
///
/// # Returns
///
/// The reconstructed IBE decryption key as a G1 affine point.
#[cfg(any(test, feature = "fuzzing"))]
pub fn test_reconstruct_ibe_from_secret_shares(
    validator_indices: &[u64],
    scalar_shares: &[Vec<Scalar>],
    weights: &[u64],
    total_weight: u64,
    identity: &[u8; 32],
) -> Result<G1Affine, IbeError> {
    if validator_indices.len() != scalar_shares.len() {
        return Err(IbeError::ValidatorIndicesSharesMismatch {
            indices_len: validator_indices.len(),
            shares_len: scalar_shares.len(),
        });
    }

    let computed_total: u64 = weights.iter().copied().sum();
    if total_weight != computed_total {
        return Err(IbeError::WeightSumMismatch {
            total_weight,
            computed_sum: computed_total,
        });
    }

    let mut virtual_player_ids: Vec<u64> = Vec::new();
    let mut dk_shares: Vec<Vec<u8>> = Vec::new();
    let h_identity = hash_to_g1(identity);

    for (vi, &validator_idx) in validator_indices.iter().enumerate() {
        let weight: u64 = weights[validator_idx as usize];
        let shares: &[Scalar] = &scalar_shares[vi];

        if shares.len() != weight as usize {
            return Err(IbeError::ShareWeightMismatch {
                validator_index: validator_idx,
                shares_count: shares.len(),
                expected_weight: weight,
            });
        }

        for (j, scalar_share) in shares.iter().enumerate() {
            let virtual_player_idx =
                weights[..validator_idx as usize].iter().sum::<u64>() + j as u64;
            virtual_player_ids.push(virtual_player_idx);

            let dk_share = h_identity.mul(scalar_share).to_affine();
            dk_shares.push(dk_share.to_compressed().to_vec());
        }
    }

    reconstruct_ibe_dk_from_g1_shares(&virtual_player_ids, &dk_shares, total_weight)
}

#[cfg(test)]
mod timelock_tests;
