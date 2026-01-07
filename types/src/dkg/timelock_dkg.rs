// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! # Timelock DKG Types
//!
//! This module defines the core types for Timelock Distributed Key Generation (DKG),
//! which is used to implement Identity-Based Encryption (IBE) for the Aptos blockchain.
//!
//! ## Overview
//!
//! Timelock DKG is a variant of standard DKG that deals **scalar values** instead of
//! group elements. This is necessary for IBE because:
//! - IBE decryption keys are scalar values in the BLS12-381 field
//! - Standard DKG deals group elements (G1Projective points)
//! - We need to encrypt and share scalars while maintaining PVSS security properties
//!
//! ## Architecture
//!
//! The timelock DKG system consists of:
//! 1. **TimelockShare**: A secret scalar share held by each validator
//! 2. **TimelockSecret**: The reconstructed master secret (scalar)
//! 3. **TimelockDKG**: The DKG protocol implementation (in `dkg/src/timelock_dkg.rs`)
//!
//! ## Cryptographic Foundation
//!
//! - **Field**: BLS12-381 scalar field (Fr)
//! - **Group**: BLS12-381 G1 curve
//! - **Pairing**: Used for PVSS verification
//! - **Encryption**: Scalars are encrypted using ElGamal-style encryption in the exponent
//!
//! ## Security Model
//!
//! - Shares are distributed using PVSS (Publicly Verifiable Secret Sharing)
//! - Each validator receives encrypted shares that only they can decrypt
//! - The master secret can be reconstructed with a threshold of shares
//! - Public verification ensures all shares are correctly formed
//!
//! ## Usage Flow
//!
//! 1. **Setup**: Validators run DKG to generate a master public key (MPK)
//! 2. **Dealing**: Each dealer creates a `TimelockTranscript` containing encrypted shares
//! 3. **Aggregation**: Transcripts are combined to form the final MPK
//! 4. **Decryption**: Each validator decrypts their `TimelockShare` from the transcript
//! 5. **Reconstruction**: Threshold shares are combined to reveal the master secret
//!
//! ## Integration with Standard DKG
//!
//! Timelock DKG runs in parallel with standard DKG:
//! - Standard DKG: Produces group element shares for randomness
//! - Timelock DKG: Produces scalar shares for IBE
//! - Both use the same PVSS infrastructure and validator set

use aptos_crypto::{CryptoMaterialError, ValidCryptoMaterial, ValidCryptoMaterialStringExt};
use aptos_crypto_derive::{DeserializeKey, SerializeKey, SilentDebug, SilentDisplay};
use aptos_dkg::pvss::{
    das::PublicParameters as DasPP,
    input_secret::InputSecret,
    traits::{Convert, Reconstructable},
    Player, WeightedConfig,
};
use serde::{Deserialize, Serialize};

// Access types from blstrs crate
use blstrs::{G1Projective, Scalar};
use ff::PrimeField;
use group::Group;

/// A Timelock share represents a validator's secret share of the master IBE secret key.
///
/// ## Purpose
///
/// Each validator receives one or more `TimelockShare` values during DKG. These shares
/// can later be combined (with threshold participants) to reconstruct the master secret,
/// which is used to derive IBE decryption keys for specific identities.
///
/// ## Structure
///
/// - `scalar`: The actual secret scalar value (element of BLS12-381 Fr field)
/// - `comm`: Cached commitment g^scalar for verification (G1 point)
///
/// ## Security Properties
///
/// - The scalar is the sensitive cryptographic material
/// - The commitment allows public verification without revealing the scalar
/// - Shares are encrypted during transmission using PVSS
/// - Only the designated validator can decrypt their shares
///
/// ## Serialization
///
/// Shares are serialized as 32-byte little-endian scalar values. The commitment
/// is recomputed on deserialization to save space.
///
/// ## Usage
///
/// ```ignore
/// // Create a share from a scalar
/// let scalar = Scalar::from(42u64);
/// let share = TimelockShare::new(scalar);
///
/// // Access the scalar value
/// let s = share.as_scalar();
///
/// // Verify the commitment
/// let comm = share.as_group_element();
/// assert_eq!(*comm, G1Projective::generator() * scalar);
/// ```
#[derive(DeserializeKey, SerializeKey, SilentDisplay, SilentDebug, PartialEq, Clone)]
pub struct TimelockShare {
    /// The secret scalar share value (BLS12-381 Fr field element)
    pub(crate) scalar: Scalar,

    /// Cached public commitment: g^scalar (G1 point)
    /// This allows verification that the share corresponds to a public commitment
    /// without revealing the scalar value.
    pub(crate) comm: G1Projective,
}

impl TimelockShare {
    /// Creates a new TimelockShare from a scalar value.
    ///
    /// The commitment is automatically computed as g^scalar where g is the
    /// G1 generator point.
    ///
    /// # Arguments
    ///
    /// * `scalar` - The secret scalar value for this share
    ///
    /// # Returns
    ///
    /// A new TimelockShare with the scalar and its commitment
    pub fn new(scalar: Scalar) -> Self {
        let comm = G1Projective::generator() * scalar;
        TimelockShare { scalar, comm }
    }

    /// Returns a reference to the secret scalar value.
    ///
    /// # Security Note
    ///
    /// This exposes the secret share value and should only be used when
    /// necessary (e.g., for reconstruction or serialization).
    pub fn as_scalar(&self) -> &Scalar {
        &self.scalar
    }

    /// Returns a reference to the public commitment (g^scalar).
    ///
    /// The commitment can be safely shared publicly for verification purposes.
    pub fn as_group_element(&self) -> &G1Projective {
        &self.comm
    }
}

/// Implementation of ValidCryptoMaterial for TimelockShare.
///
/// This allows TimelockShare to be serialized/deserialized using the aptos-crypto
/// framework, which provides hex encoding and other utilities.
///
/// ## Serialization Format
///
/// Shares are serialized as 32-byte little-endian representations of the scalar value.
/// The commitment (g^scalar) is NOT serialized - it is recomputed on deserialization.
impl ValidCryptoMaterial for TimelockShare {
    /// Empty prefix for AIP-80 compatibility (not used for timelock shares)
    const AIP_80_PREFIX: &'static str = "";

    /// Serializes the share to bytes (32-byte scalar in little-endian format)
    fn to_bytes(&self) -> Vec<u8> {
        self.scalar.to_bytes_le().to_vec()
    }
}

/// Deserialization from bytes for TimelockShare.
///
/// ## Process
///
/// 1. Validates input is exactly 32 bytes
/// 2. Attempts to parse as a valid BLS12-381 scalar
/// 3. Recomputes the commitment g^scalar
/// 4. Returns the reconstructed TimelockShare
///
/// ## Errors
///
/// Returns `CryptoMaterialError::DeserializationError` if:
/// - Input is not 32 bytes
/// - Bytes don't represent a valid scalar (e.g., value >= field modulus)
impl TryFrom<&[u8]> for TimelockShare {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> std::result::Result<TimelockShare, Self::Error> {
        // blstrs::Scalar requires exactly 32 bytes
        let bytes_array: [u8; 32] = bytes
            .try_into()
            .map_err(|_| CryptoMaterialError::DeserializationError)?;

        // Parse as scalar (validates it's in the field)
        let s = Option::<Scalar>::from(Scalar::from_repr(bytes_array))
            .ok_or(CryptoMaterialError::DeserializationError)?;

        // Reconstruct the share (recomputes commitment)
        Ok(TimelockShare::new(s))
    }
}

/// Reconstructable trait implementation for TimelockShare.
///
/// This trait is required by the DKG framework but reconstruction is NOT implemented
/// for individual shares. Instead, reconstruction happens at the `TimelockSecret` level
/// using multiple shares from different validators.
///
/// ## Why Not Implemented
///
/// - Individual shares cannot be reconstructed from sub-shares
/// - Reconstruction requires combining shares from multiple validators
/// - See `TimelockSecret::reconstruct` for the actual reconstruction logic
impl Reconstructable<WeightedConfig> for TimelockShare {
    type Share = TimelockShare;

    /// Panics if called - reconstruction not supported at share level
    fn reconstruct(_sc: &WeightedConfig, _shares: &Vec<(Player, Self::Share)>) -> Self {
        panic!("TimelockShare reconstruction not implemented - use TimelockSecret::reconstruct")
    }
}

/// The master secret for Timelock IBE, reconstructed from validator shares.
///
/// ## Purpose
///
/// `TimelockSecret` represents the master secret key for the IBE system. It is:
/// - Generated during DKG setup
/// - Never stored in full - only shares are kept by validators
/// - Reconstructed when needed using threshold shares
/// - Used to derive identity-specific decryption keys
///
/// ## Structure
///
/// Contains a single `Scalar` value which is the master secret in the BLS12-381 field.
///
/// ## Lifecycle
///
/// 1. **Generation**: Created during DKG from random input
/// 2. **Sharing**: Split into shares distributed to validators
/// 3. **Reconstruction**: Validators combine threshold shares to recover it
/// 4. **Usage**: Derive IBE decryption keys for specific identities
/// 5. **Disposal**: Immediately discarded after use
///
/// ## Security
///
/// - Must be kept secret - compromise allows decrypting all timelocks
/// - Should only exist in memory temporarily during reconstruction
/// - Threshold property ensures no single validator can reconstruct it alone
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelockSecret(pub Scalar);

/// Conversion from InputSecret to TimelockSecret.
///
/// This trait implementation allows the DKG framework to convert the random
/// input secret (generated during dealing) into the timelock-specific secret format.
///
/// ## Process
///
/// Extracts the `secret_a` component from the InputSecret, which is the scalar
/// value used for IBE. The `secret_b` component (used for standard DKG) is ignored.
impl Convert<TimelockSecret, DasPP> for InputSecret {
    fn to(&self, _pp: &DasPP) -> TimelockSecret {
        TimelockSecret(*self.get_secret_a())
    }
}

/// Reconstructable trait for TimelockSecret.
///
/// This defines how to reconstruct the master secret from validator shares.
///
/// ## Implementation Note
///
/// Currently panics - actual reconstruction logic is in `dkg/src/timelock_dkg.rs`
/// in the `TimelockDKG::reconstruct_secret_from_shares` method.
///
/// ## Future Work
///
/// This could be implemented to perform Lagrange interpolation over the shares,
/// but for now reconstruction is handled by the DKG implementation.
impl Reconstructable<WeightedConfig> for TimelockSecret {
    type Share = Vec<TimelockShare>;

    /// Panics if called - use TimelockDKG::reconstruct_secret_from_shares instead
    fn reconstruct(_sc: &WeightedConfig, _shares: &Vec<(Player, Self::Share)>) -> Self {
        panic!("TimelockSecret reconstruction not implemented - use TimelockDKG::reconstruct_secret_from_shares");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ff::Field;

    #[test]
    fn test_timelock_share_creation() {
        let scalar = Scalar::from(42u64);
        let share = TimelockShare::new(scalar);

        assert_eq!(*share.as_scalar(), scalar);

        // Verify commitment is g^scalar
        let expected_comm = G1Projective::generator() * scalar;
        assert_eq!(*share.as_group_element(), expected_comm);
    }

    #[test]
    fn test_timelock_share_serialization_roundtrip() {
        // Use a deterministic scalar value
        let scalar = Scalar::from(123456789u64);
        let share = TimelockShare::new(scalar);

        // Serialize
        let bytes = share.to_bytes();
        assert_eq!(bytes.len(), 32);

        // Deserialize
        let share2 = TimelockShare::try_from(bytes.as_slice()).unwrap();

        assert_eq!(share, share2);
        assert_eq!(*share.as_scalar(), *share2.as_scalar());
    }

    #[test]
    fn test_timelock_share_invalid_deserialization() {
        // Too short
        let short_bytes = vec![0u8; 16];
        assert!(TimelockShare::try_from(short_bytes.as_slice()).is_err());

        // Too long
        let long_bytes = vec![0u8; 64];
        assert!(TimelockShare::try_from(long_bytes.as_slice()).is_err());

        // Invalid scalar (all 0xFF is out of field)
        let invalid_bytes = vec![0xFFu8; 32];
        assert!(TimelockShare::try_from(invalid_bytes.as_slice()).is_err());
    }

    #[test]
    fn test_timelock_share_zero() {
        let zero = Scalar::ZERO;
        let share = TimelockShare::new(zero);

        assert_eq!(*share.as_scalar(), Scalar::ZERO);
        assert_eq!(*share.as_group_element(), G1Projective::identity());
    }

    #[test]
    fn test_timelock_share_one() {
        let one = Scalar::ONE;
        let share = TimelockShare::new(one);

        assert_eq!(*share.as_scalar(), Scalar::ONE);
        assert_eq!(*share.as_group_element(), G1Projective::generator());
    }

    #[test]
    fn test_timelock_secret_serialization() {
        // Use a deterministic scalar value
        let scalar = Scalar::from(987654321u64);
        let secret = TimelockSecret(scalar);

        // Test BCS serialization
        let bytes = bcs::to_bytes(&secret).unwrap();
        let secret2: TimelockSecret = bcs::from_bytes(&bytes).unwrap();

        assert_eq!(secret, secret2);
    }

    #[test]
    fn test_multiple_shares_different_scalars() {
        let share1 = TimelockShare::new(Scalar::from(1u64));
        let share2 = TimelockShare::new(Scalar::from(2u64));
        let share3 = TimelockShare::new(Scalar::from(1u64));

        assert_ne!(share1, share2);
        assert_eq!(share1, share3);
    }

    #[test]
    fn test_timelock_share_to_hex_string() {
        let scalar = Scalar::from(42u64);
        let share = TimelockShare::new(scalar);

        // Test hex encoding
        let hex = share.to_encoded_string().unwrap();
        let share2 = TimelockShare::from_encoded_string(&hex).unwrap();

        assert_eq!(share, share2);
    }
}
