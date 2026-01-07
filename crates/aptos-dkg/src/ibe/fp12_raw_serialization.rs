//! Raw Fp12 serialization matching TypeScript @noble/curves format
//!
//! This module provides Fp12 serialization that matches the TypeScript
//! bls12_381.fields.Fp12.toBytes() format exactly.
//!
//! The @noble/curves library serializes Fp12 as 576 bytes in big-endian format
//! with a specific coefficient ordering. We replicate this format exactly
//! to ensure cross-language compatibility.

use anyhow::Result;
use blstrs::{Fp, Fp12, Fp2};

/// Serialize an Fp element to 48 bytes (big-endian).
fn serialize_fp(fp: &Fp) -> [u8; 48] {
    fp.to_bytes_be()
}

/// Serialize an Fp2 element to 96 bytes.
/// Fp2 = (c0: Fp, c1: Fp) -> c0 || c1 (each 48 bytes, big-endian)
fn serialize_fp2(fp2: &Fp2) -> [u8; 96] {
    let mut result = [0u8; 96];
    result[0..48].copy_from_slice(&serialize_fp(&fp2.c0()));
    result[48..96].copy_from_slice(&serialize_fp(&fp2.c1()));
    result
}

/// Serialize Fp12 to 576 bytes matching TypeScript @noble/curves format.
///
/// Fp12 structure (via tower extension):
/// - Fp12 = (c0: Fp6, c1: Fp6)
/// - Fp6 = (c0: Fp2, c1: Fp2, c2: Fp2)
/// - Fp2 = (c0: Fp, c1: Fp)
///
/// Output format (576 bytes):
/// fp12.c0.c0 || fp12.c0.c1 || fp12.c0.c2 || fp12.c1.c0 || fp12.c1.c1 || fp12.c1.c2
/// (each Fp2 is 96 bytes = 2 * 48 byte Fp elements in big-endian)
pub fn serialize_fp12_raw(fp12: &Fp12) -> Result<Vec<u8>> {
    let mut result = Vec::with_capacity(576);

    // Serialize c0 (Fp6 = 3 Fp2 = 288 bytes)
    let c0 = fp12.c0();
    result.extend_from_slice(&serialize_fp2(&c0.c0()));
    result.extend_from_slice(&serialize_fp2(&c0.c1()));
    result.extend_from_slice(&serialize_fp2(&c0.c2()));

    // Serialize c1 (Fp6 = 3 Fp2 = 288 bytes)
    let c1 = fp12.c1();
    result.extend_from_slice(&serialize_fp2(&c1.c0()));
    result.extend_from_slice(&serialize_fp2(&c1.c1()));
    result.extend_from_slice(&serialize_fp2(&c1.c2()));

    debug_assert_eq!(result.len(), 576);
    Ok(result)
}

// Note: Deserialization is not needed for the IBE key derivation use case.
// If needed in the future, it would require access to Fp6::new() which is not
// exported by blstrs. This could be worked around using BCS deserialization
// or by contributing a change to blstrs to export Fp6.

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_crypto::blstrs::multi_pairing;
    use blstrs::{G1Projective, G2Projective};
    use group::Group;
    use std::iter;

    #[test]
    fn test_fp12_serialization_size() {
        let g1 = G1Projective::generator();
        let g2 = G2Projective::generator();
        let gt = multi_pairing(iter::once(&g1), iter::once(&g2));
        let fp12: Fp12 = gt.into();

        let bytes = serialize_fp12_raw(&fp12).expect("Serialization should work");
        assert_eq!(bytes.len(), 576, "Fp12 should serialize to 576 bytes");
    }

    #[test]
    fn test_fp12_first_bytes_match_typescript() {
        // e(G1_generator, G2_generator) should produce known first bytes
        // TypeScript output: 1250ebd871fc0a92a7b2d83168d0d727272d441befa15c503dd8e90ce98db3e7...
        let g1 = G1Projective::generator();
        let g2 = G2Projective::generator();
        let gt = multi_pairing(iter::once(&g1), iter::once(&g2));
        let fp12: Fp12 = gt.into();

        let bytes = serialize_fp12_raw(&fp12).expect("Serialization should work");

        // Expected first bytes from TypeScript (big-endian)
        let expected_start: [u8; 4] = [0x12, 0x50, 0xeb, 0xd8];
        assert_eq!(
            &bytes[0..4],
            &expected_start,
            "First 4 bytes should match TypeScript output"
        );
    }

    #[test]
    fn test_fp12_serialization_deterministic() {
        let g1 = G1Projective::generator();
        let g2 = G2Projective::generator();
        let gt = multi_pairing(iter::once(&g1), iter::once(&g2));
        let fp12: Fp12 = gt.into();

        let bytes1 = serialize_fp12_raw(&fp12).expect("Serialization should work");
        let bytes2 = serialize_fp12_raw(&fp12).expect("Serialization should work");

        assert_eq!(bytes1, bytes2, "Serialization should be deterministic");
    }
}
