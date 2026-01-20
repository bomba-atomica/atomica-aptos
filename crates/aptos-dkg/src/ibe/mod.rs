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

pub mod ciphertext;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod golden_vectors;

pub use ciphertext::Ciphertext;

use crate::utils::random::random_scalar_from_uniform_bytes;
use aptos_crypto::blstrs::SCALAR_NUM_BYTES;
use blstrs::{pairing, G1Affine, G1Projective, G2Affine, G2Projective, Scalar};
use group::{Curve, Group};
use sha3::{Digest, Sha3_256};
use std::ops::Mul;

/// Domain separation tag for IBE identity hashing.
/// Ensures unique hash domains for IBE vs other protocols.
pub const IBE_IDENTITY_DST: &[u8] = b"APTOS_IBE_IDENTITY_DST";

/// Domain separation tag for symmetric key derivation from pairing result.
/// Prevents key confusion between IBE and other uses of the pairing output.
pub const IBE_KEY_DERIVATION_DST: &[u8] = b"APTOS_IBE_KEY_DERIVATION_DST";

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
