// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Native function for IBE (Identity-Based Encryption) decryption key reconstruction.
//!
//! This module provides the Move VM native function for reconstructing IBE decryption keys
//! from PVSS threshold shares. It serves as a bridge between the Move layer and the Rust SDK.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                    Move VM IBE DK Reconstruction Flow                       │
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │                                                                              │
//! │  Move Script:                                                               │
//! │  ┌─────────────────────────────────────────────────────────────────────┐    │
//! │  │  ibe::reconstruct_ibe_dk<G1>(                                        │    │
//! │  │      validator_indices,      // vector<u64>                          │    │
//! │  │      scalar_shares,          // vector<vector<u8>> (32-byte scalars) │    │
//! │  │      weights,                // vector<u64>                          │    │
//! │  │      threshold,              // u64                                  │    │
//! │  │      total_weight,           // u64                                  │    │
//! │  │      identity                // vector<u8> (32 bytes)                │    │
//! │  │  ) -> vector<u8> (48-byte compressed G1)                             │    │
//! │  └─────────────────────────────────────────────────────────────────────┘    │
//! │                                    │                                        │
//! │                                    ▼                                        │
//! │  Native Function (this module):                                             │
//! │  1. Deserialize each inner vector from 32-byte little-endian to Scalar      │
//! │  2. Wrap each Scalar in inner Vec<Vec<Scalar>> (matches apt-dkg API)        │
//! │  3. Call apt_dkg::ibe::reconstruct_ibe_dk() [delegation point]             │
//! │  4. Serialize result to 48-byte compressed G1 and return                   │
//! │                                                                              │
//! └─────────────────────────────────────────────────────────────────────────────┘
//!
//! # Data Format (Ground Truth)
//!
//! The Move VM can only pass `vector<vector<u8>>` to native functions. This matches
//! what `aptos_dkg::ibe::reconstruct_ibe_dk()` expects:
//!
//! - `validator_indices`: Vector of validator indices (0-based)
//! - `scalar_shares`: Vec<Vec<Scalar> where each inner Vec has count = validator's weight
//! - `weights`: Full weights for ALL validators in the network
//! - `total_weight`: Sum of all validator weights
//! - `identity`: 32-byte IBE identity (from compute_identity)
//!
//! Each scalar is serialized as 32 bytes, little-endian.
//! Each G1 result is serialized as 48 bytes, compressed form.
//!
//! # Security Considerations
//!
//! - The actual cryptographic operations are performed by `aptos-dkg` to ensure
//!   consistency with the Rust SDK implementation.
//! - This native function only handles data parsing and serialization.
//! - Input validation ensures shares match the expected weights.
//!
//! # Testing
//!
//! See [ibe-test-plan.md](ibe-test-plan.md) for detailed test strategy and coverage.

use crate::natives::cryptography::algebra::abort_invariant_violated;
use aptos_dkg::ibe::reconstruct_ibe_dk;
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, safely_pop_vec_arg, SafeNativeContext, SafeNativeError, SafeNativeResult,
};
use blstrs::Scalar;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

/// Reconstructs an IBE decryption key from PVSS threshold scalar shares.
///
/// This is the main native function entry point called from Move code.
/// It deserializes scalar shares from byte vectors, delegates to `aptos-dkg`
/// for the cryptographic reconstruction, and returns the result as serialized bytes.
///
/// # Arguments
///
/// * `context` - The SafeNativeContext providing gas charging
/// * `ty_args` - Type arguments (must contain exactly one: G1)
/// * `args` - Arguments pushed from Move:
///   1. `identity` (Vec<u8>) - 32-byte IBE identity
///   2. `total_weight` (u64) - Sum of all validator weights
///   3. `threshold` (u64) - Minimum shares required (unused but kept for API compatibility)
///   4. `weights` (Vec<u64>) - Full weights for ALL validators in network
///   5. `scalar_shares` (Vec<Vec<u8>>) - 32-byte little-endian scalars per validator
///   6. `validator_indices` (Vec<u64>) - Indices of participating validators
///
/// # Returns
///
/// Returns `SafeNativeResult<SmallVec<[Value; 1]>>` containing:
/// - `Value::vector<u8>` - 48-byte compressed G1 (the reconstructed DK)
///
/// # Errors
///
/// Returns `SafeNativeError::Abort` with:
/// - Invariant violation if arguments fail validation or crypto operations fail
///
/// # Implementation Details
///
/// 1. **Argument Parsing**: Extracts and validates all arguments from the Move VM
/// 2. **Scalar Deserialization**: Each inner Vec<u8> is 32-byte little-endian Scalar
/// 3. **Format Conversion**: Wraps each Scalar in inner Vec<Vec<Scalar>> for apt-dkg
/// 4. **Delegation**: Calls `aptos_dkg::ibe::reconstruct_ibe_dk()` for crypto operations
/// 5. **Serialization**: Returns result as 48-byte compressed G1
///
/// # Gas Charging
///
/// Charges `ALGEBRA_ARK_BLS12_381_G1_PROJ_SCALAR_MUL` for the scalar multiplication
/// required in the DK derivation step.
pub fn reconstruct_ibe_dk_internal(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(
        ty_args.len(),
        1,
        "IBE DK reconstruction requires exactly one type argument: G1"
    );

    let identity: Vec<u8> = safely_pop_arg!(args, Vec<u8>);
    let total_weight: u64 = safely_pop_arg!(args, u64);
    let _threshold: u64 = safely_pop_arg!(args, u64);
    let weights: Vec<u64> = safely_pop_arg!(args, Vec<u64>);
    let scalar_shares_bytes: Vec<Vec<u8>> = safely_pop_vec_arg!(args, Vec<u8>);
    let validator_indices: Vec<u64> = safely_pop_arg!(args, Vec<u64>);

    context.charge(ALGEBRA_ARK_BLS12_381_G1_PROJ_SCALAR_MUL)?;

    let mut scalar_shares: Vec<Vec<Scalar>> = Vec::with_capacity(scalar_shares_bytes.len());

    for share_bytes in scalar_shares_bytes.iter() {
        if share_bytes.len() != 32 {
            return Err(SafeNativeError::InvariantViolation(
                abort_invariant_violated().with_message(format!(
                    "Scalar share must be 32 bytes, got {}",
                    share_bytes.len()
                )),
            ));
        }

        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(share_bytes);
        let scalar = Scalar::from_bytes_le(&bytes).unwrap();

        scalar_shares.push(vec![scalar]);
    }

    let identity_array: [u8; 32] = identity
        .try_into()
        .map_err(|_| abort_invariant_violated())?;

    let reconstructed_dk: blstrs::G1Affine = match reconstruct_ibe_dk(
        &validator_indices,
        &scalar_shares,
        &weights,
        total_weight,
        &identity_array,
    ) {
        Ok(dk) => dk,
        Err(e) => {
            return Err(SafeNativeError::InvariantViolation(
                abort_invariant_violated()
                    .with_message(format!("IBE DK reconstruction failed: {}", e)),
            ));
        },
    };

    let dk_bytes = reconstructed_dk.to_compressed().to_vec();

    Ok(smallvec![Value::vector_u8(dk_bytes)])
}
