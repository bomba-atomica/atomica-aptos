// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Identity-Based Encryption (IBE) implementation for timelock encryption.
//!
//! This module implements Boneh-Franklin IBE using BLS12-381 curves.
//! The implementation is designed for the Atomica sealed bid protocol where:
//! - Master Public Key (MPK): G2 point (96 bytes compressed)
//! - Decryption Key (DK): G1 point (48 bytes compressed)
//! - Identity: Arbitrary bytes (e.g., interval number)
//!
//! # Security Model
//! - MPK is generated via threshold DKG by validators
//! - Decryption keys are revealed only after the timelock period
//! - Uses pairing-based cryptography: e(G1, G2) -> Gt

pub mod errors;
mod gt_serialization_fix;
#[cfg(test)]
mod gt_serialization_test;

use crate::weighted_vuf::bls::BLS_WVUF_DST;
use anyhow::anyhow;
use aptos_crypto::blstrs::{multi_pairing, random_scalar};
use blstrs::{G1Projective, G2Projective, Gt, Scalar};
use errors::Result;
use group::Group;
use rand::thread_rng;
use sha3::{Digest, Keccak256};
use std::iter;

/// Ciphertext produced by IBE encryption.
///
/// Structure: (U, V) where:
/// - U = r * G2_generator (randomness commitment)
/// - V = M XOR H(e(Q_id, MPK)^r) (encrypted message)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ciphertext {
    /// U component: r * G2_generator
    pub u: G2Projective,
    /// V component: encrypted message bytes
    pub v: Vec<u8>,
}

/// Encrypts a message using Identity-Based Encryption.
///
/// # Arguments
/// * `mpk` - Master Public Key (G2 point from DKG)
/// * `identity` - Identity bytes (e.g., sha256(interval || chain_id))
/// * `message` - Plaintext message to encrypt
///
/// # Returns
/// Ciphertext that can only be decrypted with the corresponding decryption key
///
/// # Example
/// ```ignore
/// let mpk = ...; // From blockchain
/// let identity = compute_timelock_identity(interval, chain_id);
/// let bid_data = b"secret_bid_100_tokens";
/// let ciphertext = ibe_encrypt(&mpk, &identity, bid_data)?;
/// ```
#[allow(dead_code)]
pub fn ibe_encrypt(mpk: &G2Projective, identity: &[u8], message: &[u8]) -> Result<Ciphertext> {
    // Boneh-Franklin IBE encryption:
    // C = <r*P, M XOR H(e(Q_ID, P_pub)^r)>
    // where P = G2_generator, P_pub = MPK (G2), Q_ID = H(ID) (G1)

    // 1. Generate random scalar r using secure RNG
    let mut rng = thread_rng();
    let r = random_scalar(&mut rng);

    // 2. Compute U = r * G2_generator
    let u = G2Projective::generator() * r;

    // 3. Hash identity to G1 curve point: Q_id = H(identity)
    let q_id = G1Projective::hash_to_curve(identity, BLS_WVUF_DST, b"H(m)");

    // 4. Compute gid = e(Q_id, MPK)^r
    // We compute e(Q_id, MPK) first, then raise to r
    let pair = multi_pairing(iter::once(&q_id), iter::once(mpk));
    let gid = pair * r;

    // 5. Derive symmetric key K = H(gid)
    let key_hash = hash_gt_to_bytes(&gid)?;

    // 6. Encrypt message: V = M XOR K
    let v = xor_bytes(message, &key_hash);

    // 7. Return ciphertext
    Ok(Ciphertext { u, v })
}

/// Decrypts a ciphertext using the decryption key.
///
/// # Arguments
/// * `dk` - Decryption key (G1 point = H(identity)^msk)
/// * `ciphertext` - Ciphertext to decrypt
///
/// # Returns
/// Plaintext message bytes
///
/// # Example
/// ```ignore
/// let dk = ...; // From blockchain after reveal
/// let plaintext = ibe_decrypt(&dk, &ciphertext)?;
/// ```
#[allow(dead_code)]
pub fn ibe_decrypt(dk: &G1Projective, ciphertext: &Ciphertext) -> Result<Vec<u8>> {
    // Boneh-Franklin IBE decryption:
    // Recover symmetric key via pairing and decrypt

    // 1. Compute gid = e(DK, U) = e(s*Q_id, r*P) = e(Q_id, P)^(sr)
    let gid = multi_pairing(iter::once(dk), iter::once(&ciphertext.u));

    // 2. Derive symmetric key K = H(gid)
    let key_hash = hash_gt_to_bytes(&gid)?;

    // 3. Decrypt message: M = V XOR K
    let plaintext = xor_bytes(&ciphertext.v, &key_hash);

    // 4. Return plaintext
    Ok(plaintext)
}

/// Derives a decryption key for a specific identity.
///
/// This is typically done by validators during the reveal phase.
///
/// # Arguments
/// * `msk` - Master Secret Key (from DKG)
/// * `identity` - Identity bytes
///
/// # Returns
/// Decryption key (G1 point)
#[allow(dead_code)]
pub fn derive_decryption_key(msk: &Scalar, identity: &[u8]) -> Result<G1Projective> {
    // IBE key derivation: DK = msk * H(identity)

    // 1. Hash identity to G1 curve point: Q_id = H(identity)
    let q_id = G1Projective::hash_to_curve(identity, BLS_WVUF_DST, b"H(m)");

    // 2. Compute decryption key: DK = msk * Q_id
    let dk = q_id * msk;

    // 3. Return decryption key
    Ok(dk)
}

/// Serializes a G2 point to compressed bytes (96 bytes).
///
/// # Arguments
/// * `point` - G2 point to serialize
///
/// # Returns
/// 96-byte compressed representation
#[allow(dead_code)]
pub fn serialize_g2(point: &G2Projective) -> Result<Vec<u8>> {
    // Use blstrs compressed serialization (96 bytes for G2)
    Ok(point.to_compressed().to_vec())
}

/// Deserializes a G2 point from compressed bytes.
///
/// # Arguments
/// * `bytes` - 96-byte compressed representation
///
/// # Returns
/// G2 point
#[allow(dead_code)]
pub fn deserialize_g2(bytes: &[u8]) -> Result<G2Projective> {
    // Validate input length
    if bytes.len() != 96 {
        return Err(anyhow!(
            "Invalid G2 compressed bytes length: expected 96, got {}",
            bytes.len()
        ));
    }

    // Convert to fixed-size array
    let mut bytes_array = [0u8; 96];
    bytes_array.copy_from_slice(bytes);

    // Deserialize using blstrs
    let point_option = G2Projective::from_compressed(&bytes_array);

    // Check if deserialization succeeded (point is on curve)
    if point_option.is_some().unwrap_u8() == 1u8 {
        Ok(point_option.unwrap())
    } else {
        Err(anyhow!("Invalid G2 point: not on curve or malformed"))
    }
}

/// Serializes a G1 point to compressed bytes (48 bytes).
#[allow(dead_code)]
pub fn serialize_g1(point: &G1Projective) -> Result<Vec<u8>> {
    // Use blstrs compressed serialization (48 bytes for G1)
    Ok(point.to_compressed().to_vec())
}

/// Deserializes a G1 point from compressed bytes.
#[allow(dead_code)]
pub fn deserialize_g1(bytes: &[u8]) -> Result<G1Projective> {
    // Validate input length
    if bytes.len() != 48 {
        return Err(anyhow!(
            "Invalid G1 compressed bytes length: expected 48, got {}",
            bytes.len()
        ));
    }

    // Convert to fixed-size array
    let mut bytes_array = [0u8; 48];
    bytes_array.copy_from_slice(bytes);

    // Deserialize using blstrs
    let point_option = G1Projective::from_compressed(&bytes_array);

    // Check if deserialization succeeded (point is on curve)
    if point_option.is_some().unwrap_u8() == 1u8 {
        Ok(point_option.unwrap())
    } else {
        Err(anyhow!("Invalid G1 point: not on curve or malformed"))
    }
}

/// Hashes a Gt element to bytes for use as a symmetric key.
///
/// # Arguments
/// * `gt` - Gt element from pairing
///
/// # Returns
/// Key bytes (32 bytes for XOR)
///
/// # Implementation Note
/// FIXED (Bug #4): Now uses proper Fp12 serialization (576 bytes) instead of debug format.
/// This ensures cross-language compatibility with TypeScript @noble/curves implementation
/// which uses Fp12.toBytes(). The serialization is deterministic and matches the standard
/// BCS format for Fp12 elements.
#[allow(dead_code)]
fn hash_gt_to_bytes(gt: &Gt) -> Result<Vec<u8>> {
    // Use the fixed serialization from gt_serialization_fix module
    gt_serialization_fix::hash_gt_to_bytes(gt)
}

/// XORs two byte slices, cycling the second if shorter.
#[allow(dead_code)]
fn xor_bytes(a: &[u8], b: &[u8]) -> Vec<u8> {
    a.iter()
        .zip(b.iter().cycle())
        .map(|(&x, &y)| x ^ y)
        .collect()
}

/// Computes the canonical timelock identity for a given interval.
///
/// Format: sha3_256(interval_u64_le || chain_id_u8 || "atomica_timelock")
///
/// # Arguments
/// * `interval` - Timelock interval number
/// * `chain_id` - Chain ID (to prevent cross-chain replay)
///
/// # Returns
/// 32-byte identity for IBE encryption
///
/// # Example
/// ```ignore
/// let identity = compute_timelock_identity(1000, 1);
/// // identity will be a deterministic 32-byte hash
/// ```
#[allow(dead_code)]
pub fn compute_timelock_identity(interval: u64, chain_id: u8) -> Vec<u8> {
    // Construct canonical identity using Keccak256 (SHA3-256)
    let mut hasher = Keccak256::new();

    // Add interval as little-endian bytes
    hasher.update(interval.to_le_bytes());

    // Add chain ID
    hasher.update([chain_id]);

    // Add domain separator to prevent collisions
    hasher.update(b"atomica_timelock");

    // Return 32-byte hash as identity
    hasher.finalize().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ibe_encrypt_decrypt_roundtrip() {
        use aptos_crypto::blstrs::random_scalar;
        use rand::thread_rng;

        // 1. Generate test MSK and derive MPK
        let mut rng = thread_rng();
        let msk = random_scalar(&mut rng);
        let mpk = G2Projective::generator() * msk;

        // 2. Test identity and message
        let identity = b"test_identity_block_1000";
        let message = b"secret_bid_value_12345";

        // 3. Encrypt the message
        let ciphertext = ibe_encrypt(&mpk, identity, message).expect("Encryption should succeed");

        // 4. Derive decryption key for this identity
        let dk = derive_decryption_key(&msk, identity).expect("Key derivation should succeed");

        // 5. Decrypt and verify
        let decrypted = ibe_decrypt(&dk, &ciphertext).expect("Decryption should succeed");

        assert_eq!(
            message.as_slice(),
            decrypted.as_slice(),
            "Decrypted message should match original"
        );
    }

    #[test]
    fn test_serialize_deserialize_g2() {
        use aptos_crypto::blstrs::random_scalar;
        use rand::thread_rng;

        // Generate a random G2 point
        let mut rng = thread_rng();
        let scalar = random_scalar(&mut rng);
        let original_point = G2Projective::generator() * scalar;

        // Serialize
        let bytes = serialize_g2(&original_point).expect("Serialization should succeed");

        // Verify it's 96 bytes (compressed G2)
        assert_eq!(bytes.len(), 96, "G2 compressed should be 96 bytes");

        // Deserialize
        let deserialized_point = deserialize_g2(&bytes).expect("Deserialization should succeed");

        // Verify equality
        assert_eq!(
            original_point, deserialized_point,
            "Deserialized point should equal original"
        );
    }

    #[test]
    fn test_serialize_deserialize_g1() {
        use aptos_crypto::blstrs::random_scalar;
        use rand::thread_rng;

        // Generate a random G1 point
        let mut rng = thread_rng();
        let scalar = random_scalar(&mut rng);
        let original_point = G1Projective::generator() * scalar;

        // Serialize
        let bytes = serialize_g1(&original_point).expect("Serialization should succeed");

        // Verify it's 48 bytes (compressed G1)
        assert_eq!(bytes.len(), 48, "G1 compressed should be 48 bytes");

        // Deserialize
        let deserialized_point = deserialize_g1(&bytes).expect("Deserialization should succeed");

        // Verify equality
        assert_eq!(
            original_point, deserialized_point,
            "Deserialized point should equal original"
        );
    }

    #[test]
    fn test_xor_bytes() {
        let a = vec![1, 2, 3, 4];
        let b = vec![5, 6];
        let result = xor_bytes(&a, &b);
        assert_eq!(result, vec![4, 4, 6, 2]); // 1^5, 2^6, 3^5, 4^6
    }

    #[test]
    fn test_compute_timelock_identity() {
        // Test determinism: same inputs produce same output
        let identity1 = compute_timelock_identity(1000, 1);
        let identity2 = compute_timelock_identity(1000, 1);
        assert_eq!(
            identity1, identity2,
            "Same inputs should produce same identity"
        );

        // Verify output length (32 bytes from Keccak256)
        assert_eq!(identity1.len(), 32, "Identity should be 32 bytes");

        // Test different intervals produce different outputs
        let identity_interval_1000 = compute_timelock_identity(1000, 1);
        let identity_interval_2000 = compute_timelock_identity(2000, 1);
        assert_ne!(
            identity_interval_1000, identity_interval_2000,
            "Different intervals should produce different identities"
        );

        // Test different chain IDs produce different outputs
        let identity_chain_1 = compute_timelock_identity(1000, 1);
        let identity_chain_2 = compute_timelock_identity(1000, 2);
        assert_ne!(
            identity_chain_1, identity_chain_2,
            "Different chain IDs should produce different identities"
        );
    }
}
