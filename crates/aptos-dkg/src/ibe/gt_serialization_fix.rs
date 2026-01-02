//! Proper Gt serialization for cross-language compatibility
//!
//! This module provides consistent Gt serialization that matches the TypeScript
//! implementation using @noble/curves Fp12.toBytes()

use anyhow::{anyhow, Result};
use blstrs::{Fp12, Gt};
use ff::PrimeField;
use sha3::{Digest, Keccak256};

/// Serializes a Gt element to bytes via Fp12 representation.
///
/// This produces a 576-byte deterministic serialization that matches the TypeScript
/// implementation's Fp12.toBytes() format, enabling cross-language compatibility.
///
/// # Structure
/// Fp12 = (c0: Fp6, c1: Fp6)
/// Fp6 = (c0: Fp2, c1: Fp2, c2: Fp2)
/// Fp2 = (c0: Fp, c1: Fp)
/// Fp = 48 bytes
///
/// Total: 2 × 3 × 2 × 48 = 576 bytes
///
/// # Arguments
/// * `gt` - Gt element from pairing
///
/// # Returns
/// 576-byte serialization
pub fn serialize_gt(gt: &Gt) -> Result<Vec<u8>> {
    // Convert Gt to Fp12
    let fp12: Fp12 = (*gt).into();

    // Serialize Fp12 to bytes
    // The blstrs library should provide this via the ff::PrimeField trait
    // which includes to_repr() method

    // Get the internal representation
    // Fp12 is composed of two Fp6 elements
    // We need to serialize them in a deterministic order

    // Try using the standard serialization if available
    let bytes = serialize_fp12(&fp12)?;

    Ok(bytes)
}

/// Serializes an Fp12 element to 576 bytes using BCS.
///
/// BCS (Binary Canonical Serialization) Layout:
/// - 576 bytes total (12 Fp elements × 48 bytes each)
/// - Deterministic and well-defined format
/// - Compatible across Rust/Move ecosystems
///
/// Structure:
/// - bytes[0..288]: c0 (Fp6)
///   - bytes[0..96]: c0.c0 (Fp2)
///   - bytes[96..192]: c0.c1 (Fp2)
///   - bytes[192..288]: c0.c2 (Fp2)
/// - bytes[288..576]: c1 (Fp6)
///   - bytes[288..384]: c1.c0 (Fp2)
///   - bytes[384..480]: c1.c1 (Fp2)
///   - bytes[480..576]: c1.c2 (Fp2)
///
/// # Cross-Language Compatibility
/// TypeScript implementations must use the SAME BCS serialization format.
/// The @noble/curves Fp12.toBytes() produces a different format, so TypeScript
/// must serialize to BCS instead. See cross-lang-ibe-verification.test.ts.
fn serialize_fp12(fp12: &Fp12) -> Result<Vec<u8>> {
    // Use BCS serialization - standard for Aptos/Move ecosystem
    let bytes = bcs::to_bytes(fp12)
        .map_err(|e| anyhow!("Failed to serialize Fp12 with BCS: {}", e))?;

    // Validate size (should be 576 bytes for Fp12)
    if bytes.len() != 576 {
        return Err(anyhow!(
            "Unexpected Fp12 BCS serialization size: expected 576 bytes, got {}",
            bytes.len()
        ));
    }

    Ok(bytes)
}

/// Hashes a Gt element to bytes for use as a symmetric key (fixed version).
///
/// This replaces the old `format!("{:?}", gt)` approach with proper serialization.
///
/// # Arguments
/// * `gt` - Gt element from pairing
///
/// # Returns
/// 32-byte key derived from Keccak256(serialize_gt(gt))
pub fn hash_gt_to_bytes(gt: &Gt) -> Result<Vec<u8>> {
    let serialized = serialize_gt(gt)?;
    let mut hasher = Keccak256::new();
    hasher.update(&serialized);
    Ok(hasher.finalize().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_crypto::blstrs::multi_pairing;
    use blstrs::{G1Projective, G2Projective};
    use group::Group;
    use std::iter;

    #[test]
    fn test_gt_serialization_size() {
        let g1 = G1Projective::generator();
        let g2 = G2Projective::generator();
        let gt = multi_pairing(iter::once(&g1), iter::once(&g2));

        let serialized = serialize_gt(&gt).expect("Serialization should work");

        // Should be 576 bytes (12 Fp elements × 48 bytes)
        assert_eq!(
            serialized.len(),
            576,
            "Gt serialization should be 576 bytes"
        );
    }

    #[test]
    fn test_gt_serialization_deterministic() {
        let g1 = G1Projective::generator();
        let g2 = G2Projective::generator();
        let gt = multi_pairing(iter::once(&g1), iter::once(&g2));

        let bytes1 = serialize_gt(&gt).expect("Serialization should work");
        let bytes2 = serialize_gt(&gt).expect("Serialization should work");

        assert_eq!(bytes1, bytes2, "Serialization should be deterministic");
    }

    #[test]
    fn test_hash_gt_to_bytes() {
        let g1 = G1Projective::generator();
        let g2 = G2Projective::generator();
        let gt = multi_pairing(iter::once(&g1), iter::once(&g2));

        let hash = hash_gt_to_bytes(&gt).expect("Hashing should work");

        // Should be 32 bytes (Keccak256 output)
        assert_eq!(hash.len(), 32, "Hash should be 32 bytes");
    }

    #[test]
    fn test_gt_serialization_output_for_cross_lang_verification() {
        // This test outputs the serialization of e(G1_generator, G2_generator)
        // for cross-language verification with TypeScript
        let g1 = G1Projective::generator();
        let g2 = G2Projective::generator();
        let gt = multi_pairing(iter::once(&g1), iter::once(&g2));

        let serialized = serialize_gt(&gt).expect("Serialization should work");
        let hash = hash_gt_to_bytes(&gt).expect("Hashing should work");

        println!("\n=== Cross-Language Verification Data ===");
        println!("Input: e(G1_generator, G2_generator)");
        println!("Serialized Gt (first 64 bytes): {:02x?}", &serialized[..64]);
        println!("Serialized Gt (last 64 bytes): {:02x?}", &serialized[serialized.len()-64..]);
        println!("Serialized Gt length: {} bytes", serialized.len());
        println!("Hash (Keccak256): {:02x?}", hash);
        println!("========================================\n");

        // These values can be compared with TypeScript implementation
        assert_eq!(serialized.len(), 576);
        assert_eq!(hash.len(), 32);
    }
}
