// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Ciphertext structure for Identity-Based Encryption (IBE).
//!
//! The ciphertext consists of:
//! - `u`: A G2 element representing g2^r where r is a random scalar
//! - `v`: The encrypted message XOR'd with the hash of the pairing

use blstrs::{G2Affine, G2Projective};
use group::Curve;
use serde::{Deserialize, Serialize};

/// IBE ciphertext containing the encrypted message.
///
/// Structure:
/// - `u`: g2^r where r is the random encryption scalar
/// - `v`: message XOR H(e(H(identity), mpk)^r)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ciphertext {
    /// The G2 element g2^r (stored in affine form for compact serialization)
    pub u: G2Affine,
    /// The encrypted payload: message XOR derived_key
    pub v: Vec<u8>,
}

impl Ciphertext {
    /// Creates a new ciphertext from a G2 projective point and encrypted bytes.
    pub fn new(u: G2Projective, v: Vec<u8>) -> Self {
        Self {
            u: u.to_affine(),
            v,
        }
    }

    /// Returns the U component as a projective point.
    pub fn u_projective(&self) -> G2Projective {
        G2Projective::from(self.u)
    }

    /// Returns the encrypted payload.
    pub fn payload(&self) -> &[u8] {
        &self.v
    }

    /// Returns the length of the encrypted payload.
    pub fn payload_len(&self) -> usize {
        self.v.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use group::Group;

    #[test]
    fn test_ciphertext_roundtrip() {
        let u = G2Projective::generator();
        let v = vec![1, 2, 3, 4, 5];
        let ct = Ciphertext::new(u, v.clone());

        assert_eq!(ct.u_projective(), u);
        assert_eq!(ct.payload(), &v[..]);
        assert_eq!(ct.payload_len(), 5);
    }

    #[test]
    fn test_ciphertext_serialization() {
        let u = G2Projective::generator();
        let v = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let ct = Ciphertext::new(u, v);

        // Test BCS serialization
        let encoded = bcs::to_bytes(&ct).expect("serialization should succeed");
        let decoded: Ciphertext =
            bcs::from_bytes(&encoded).expect("deserialization should succeed");

        assert_eq!(ct, decoded);
    }
}
