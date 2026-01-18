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
//! # Security
//!
//! The IBE scheme relies on the Bilinear Diffie-Hellman (BDH) assumption.
//! The identity derivation includes the timelock ID and deadline to prevent
//! cross-timelock attacks.

pub mod ciphertext;

#[cfg(test)]
mod tests;

pub use ciphertext::Ciphertext;

use crate::utils::random::random_scalar_from_uniform_bytes;
use aptos_crypto::blstrs::SCALAR_NUM_BYTES;
use blstrs::{pairing, G1Affine, G1Projective, G2Affine, G2Projective, Scalar};
use group::{Curve, Group};
use sha3::{Digest, Sha3_256};
use std::ops::Mul;

/// Domain separation tag for IBE identity hashing.
pub const IBE_IDENTITY_DST: &[u8] = b"APTOS_IBE_IDENTITY_DST";

/// Domain separation tag for symmetric key derivation from pairing result.
pub const IBE_KEY_DERIVATION_DST: &[u8] = b"APTOS_IBE_KEY_DERIVATION_DST";

/// Computes an IBE identity from a timelock ID and deadline.
///
/// The identity is a 32-byte hash that uniquely identifies a timelock
/// for encryption purposes.
///
/// # Arguments
/// * `timelock_id` - Unique identifier for the timelock
/// * `deadline_us` - Deadline in microseconds since epoch
///
/// # Returns
/// A 32-byte identity hash
pub fn compute_identity(timelock_id: u64, deadline_us: u64) -> [u8; 32] {
    let mut hasher = Sha3_256::new();
    hasher.update(IBE_IDENTITY_DST);
    hasher.update(timelock_id.to_le_bytes());
    hasher.update(deadline_us.to_le_bytes());
    hasher.finalize().into()
}

/// Hashes an identity to a G1 curve point.
///
/// Uses the standard hash-to-curve construction for BLS12-381.
///
/// # Arguments
/// * `identity` - The identity bytes to hash
///
/// # Returns
/// A G1 projective point representing the hashed identity
pub fn hash_to_g1(identity: &[u8]) -> G1Projective {
    G1Projective::hash_to_curve(identity, IBE_IDENTITY_DST, b"H(id)")
}

/// Derives the decryption key for an identity given the master secret.
///
/// Computes `dk = H(identity)^secret` where H maps the identity to G1.
///
/// # Arguments
/// * `secret` - The master secret scalar (or a share of it)
/// * `identity` - The identity bytes
///
/// # Returns
/// The decryption key as a G1 affine point
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
