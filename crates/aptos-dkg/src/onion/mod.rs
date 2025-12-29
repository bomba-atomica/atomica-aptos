// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! N-Layer Onion Encryption Framework
//!
//! This module implements the "Onion" encryption scheme used for Timelock auctions.
//! It allows a message to be encrypted with multiple layers of encryption, where each
//! layer corresponds to a specific time interval (or other condition).
//!
//! The mechanism guarantees that a message M encrypted for intervals [T_1, T_2, ..., T_n]
//! can only be decrypted if the secrets for ALL corresponding intervals are available.
//!
//! # Architecture
//!
//! - `OnionEncryption`: The core trait defining the interface for onion encryption.
//! - `OnionLayer`: Represents a single layer of encryption parameters (e.g., public key for an interval).
//! - `OnionCiphertext`: The resulting encrypted message structure.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::ibe::{ibe_encrypt, ibe_decrypt, Ciphertext, serialize_g2, deserialize_g2};

/// Represents the public parameters for a single encryption layer.
/// For timelock, this typically wraps the Identity-Based Encryption (IBE) public key
/// (Master Public Key + Identity/Interval ID).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OnionPublicParams {
    /// The public key (MPK) used for this layer (Serialized G2Projective)
    pub public_key: Vec<u8>,
    /// The identity/interval ID for this layer
    pub id: Vec<u8>,
}

/// Represents the decryption key for a single encryption layer.
/// For timelock, this is the Secret Share or Aggregated Secret for an interval.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OnionSecretKey {
    /// The secret key bytes for this layer (Serialized G1Projective)
    pub secret_key: Vec<u8>,
}

/// The multi-layered ciphertext.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OnionCiphertext {
    /// The IBE ciphertext component U (serialized G2Projective) for the current layer
    pub u: Vec<u8>,
    /// The encrypted payload for the next layer (or final message)
    /// This is V = M XOR Hash(Key) in standard IBE, or SymEnc(Key, NextLayer) in Onion.
    pub v: Vec<u8>,
}

/// Traits for implementing N-Layer Onion Encryption.
pub trait OnionEncryption {
    /// Encrypt a message with N layers of protection.
    fn multi_encrypt(
        public_params: &[OnionPublicParams],
        message: &[u8],
    ) -> Result<OnionCiphertext>;

    /// Decrypt (peel) one layer of the onion using the provided secret key.
    fn decrypt_layer(
        ciphertext: &OnionCiphertext,
        secret_key: &OnionSecretKey,
    ) -> Result<DecryptionResult>;
}

/// Result of a decryption operation.
#[derive(Debug)]
pub enum DecryptionResult {
    /// Decryption successful, another layer remains.
    NextLayer(OnionCiphertext),
    /// Decryption successful, this was the final layer.
    Plaintext(Vec<u8>),
}

/// Implementation of Onion Encryption using Boneh-Franklin IBE.
pub struct IBEOnion;


impl OnionEncryption for IBEOnion {
    fn multi_encrypt(
        public_params: &[OnionPublicParams],
        message: &[u8],
    ) -> Result<OnionCiphertext> {
        let mut current_payload = message.to_vec();

        // Iterate from innermost layer to outermost layer
        for layer_params in public_params.iter().rev() {
            // Clone the MPK bytes to fallback to try_into
            let mpk_bytes = layer_params.public_key.clone();
            // Deserialize MPK (G2)
            let mpk = deserialize_g2(&mpk_bytes)?;
            
            // Encrypt current payload for this layer's identity
            let ct = ibe_encrypt(&mpk, &layer_params.id, &current_payload)?;
            
            // Serialize encryption result (Ciphertext) to bytes to become payload for next layer
            // Manual serialization since Ciphertext might not derive Serde
            let u_bytes = serialize_g2(&ct.u)?;
            let v_bytes = ct.v;
            // We use BCS to serialize the tuple (u_bytes, v_bytes)
            current_payload = bcs::to_bytes(&(u_bytes, v_bytes))?;
        }

        // The final result `current_payload` is the serialized Ciphertext of the outermost layer.
        // We need to return it as OnionCiphertext.
        // Since OnionCiphertext (my struct) is identical to (u, v), I can deserialize into it?
        // Wait, current_payload is (u_bytes, v_bytes).
        let (u, v): (Vec<u8>, Vec<u8>) = bcs::from_bytes(&current_payload)?;
        
        Ok(OnionCiphertext {
            u,
            v,
        })
    }

    fn decrypt_layer(
        ciphertext: &OnionCiphertext,
        secret_key: &OnionSecretKey,
    ) -> Result<DecryptionResult> {
        // Convert OnionCiphertext to crate::ibe::Ciphertext
        let u_point = deserialize_g2(&ciphertext.u)?;
        let ibe_ct = Ciphertext {
            u: u_point,
            v: ciphertext.v.clone(),
        };

        // Deserialize secret key (G1)
        // Need deserialize_g1 from ibe module? It was there in tests, let's assume it's pub.
        // Step 284 showed pub fn deserialize_g1.
        let dk = crate::ibe::deserialize_g1(&secret_key.secret_key)?;

        // Decrypt
        let decrypted_bytes = ibe_decrypt(&dk, &ibe_ct)?;

        // Try to interpret plaintext as next layer (u_bytes, v_bytes)
        if let Ok((u_inner, v_inner)) = bcs::from_bytes::<(Vec<u8>, Vec<u8>)>(&decrypted_bytes) {
             Ok(DecryptionResult::NextLayer(OnionCiphertext {
                 u: u_inner,
                 v: v_inner,
             }))
        } else {
             Ok(DecryptionResult::Plaintext(decrypted_bytes))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blstrs::{G1Projective, G2Projective, Scalar};
    use crate::weighted_vuf::bls::BLS_WVUF_DST;
    use group::Group;

    #[test]
    fn test_onion_encryption_flow_real_ibe() {
        let payload = b"secret_auction_bid";
        
        // Setup Keys
        let mk: Scalar = aptos_crypto::blstrs::random_scalar(&mut rand::thread_rng());
        let mpk = G2Projective::generator() * mk;
        let mpk_bytes = mpk.to_compressed().to_vec();

        // Define 3 layers [Outer, Middle, Inner]
        let id_outer = b"id_10";
        let id_middle = b"id_11";
        let id_inner = b"id_12";

        let params = vec![
            OnionPublicParams { public_key: mpk_bytes.clone(), id: id_outer.to_vec() },
            OnionPublicParams { public_key: mpk_bytes.clone(), id: id_middle.to_vec() },
            OnionPublicParams { public_key: mpk_bytes.clone(), id: id_inner.to_vec() },
        ];

        // 1. Encrypt
        let ciphertext = IBEOnion::multi_encrypt(&params, payload).expect("Encryption failed");

        // 2. Derive Secret Keys for Decryption
        // SK = H(ID) * MK
        let q_outer = G1Projective::hash_to_curve(id_outer, BLS_WVUF_DST, b"H(m)");
        let sk_outer = q_outer * mk;
        let key_outer = OnionSecretKey { secret_key: sk_outer.to_compressed().to_vec() };

        let q_middle = G1Projective::hash_to_curve(id_middle, BLS_WVUF_DST, b"H(m)");
        let sk_middle = q_middle * mk;
        let key_middle = OnionSecretKey { secret_key: sk_middle.to_compressed().to_vec() };

        let q_inner = G1Projective::hash_to_curve(id_inner, BLS_WVUF_DST, b"H(m)");
        let sk_inner = q_inner * mk;
        let key_inner = OnionSecretKey { secret_key: sk_inner.to_compressed().to_vec() };

        // 3. Decrypt Layer 1 (Outer)
        let res1 = IBEOnion::decrypt_layer(&ciphertext, &key_outer).expect("Decrypt outer failed");
        let next_ct = match res1 {
            DecryptionResult::NextLayer(ct) => ct,
            _ => panic!("Expected next layer"),
        };

        // 4. Decrypt Layer 2 (Middle)
        let res2 = IBEOnion::decrypt_layer(&next_ct, &key_middle).expect("Decrypt middle failed");
        let next_ct_2 = match res2 {
            DecryptionResult::NextLayer(ct) => ct,
            _ => panic!("Expected next layer"),
        };

        // 5. Decrypt Layer 3 (Inner)
        let res3 = IBEOnion::decrypt_layer(&next_ct_2, &key_inner).expect("Decrypt inner failed");
        let final_msg = match res3 {
            DecryptionResult::Plaintext(msg) => msg,
            _ => panic!("Expected plaintext"),
        };

        assert_eq!(final_msg, payload);
    }
}

