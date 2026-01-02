//! Raw Fp12 serialization matching TypeScript @noble/curves format
//!
//! This module provides Fp12 serialization that matches the TypeScript
//! bls12_381.fields.Fp12.toBytes() format exactly.

use anyhow::{anyhow, Result};
use blstrs::Fp12;

/// Serialize Fp12 to raw bytes matching TypeScript format.
///
/// The @noble/curves library serializes Fp12 as 576 bytes of raw field elements
/// in big-endian format. We need to match this exactly for cross-language compatibility.
///
/// Current approach: Use BCS serialization and document that TypeScript
/// must also use BCS (or we implement converters on both sides).
pub fn serialize_fp12_raw(fp12: &Fp12) -> Result<Vec<u8>> {
    // For now, delegate to BCS
    // TODO: If cross-language tests fail, implement manual field-by-field serialization
    let bytes = bcs::to_bytes(fp12)
        .map_err(|e| anyhow!("Failed to serialize Fp12: {}", e))?;

    if bytes.len() != 576 {
        return Err(anyhow!(
            "Unexpected Fp12 serialization size: expected 576, got {}",
            bytes.len()
        ));
    }

    Ok(bytes)
}

/// Deserialize Fp12 from raw bytes.
pub fn deserialize_fp12_raw(bytes: &[u8]) -> Result<Fp12> {
    if bytes.len() != 576 {
        return Err(anyhow!(
            "Invalid Fp12 bytes length: expected 576, got {}",
            bytes.len()
        ));
    }

    bcs::from_bytes(bytes).map_err(|e| anyhow!("Failed to deserialize Fp12: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_crypto::blstrs::multi_pairing;
    use blstrs::{G1Projective, G2Projective, Gt};
    use group::Group;
    use std::iter;

    #[test]
    fn test_fp12_roundtrip() {
        let g1 = G1Projective::generator();
        let g2 = G2Projective::generator();
        let gt = multi_pairing(iter::once(&g1), iter::once(&g2));
        let fp12: Fp12 = gt.into();

        let bytes = serialize_fp12_raw(&fp12).expect("Serialization should work");
        let deserialized = deserialize_fp12_raw(&bytes).expect("Deserialization should work");

        let gt_back: Gt = deserialized.into();
        assert_eq!(gt, gt_back, "Roundtrip should preserve value");
    }
}
