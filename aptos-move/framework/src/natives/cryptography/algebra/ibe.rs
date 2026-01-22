// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Native function for IBE (Identity-Based Encryption) decryption key reconstruction.
//!
//! This module provides the Move VM native function for reconstructing IBE decryption keys
//! from threshold shares. It serves as a bridge between the Move layer and the Rust SDK.
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
//! │  │      scalar_shares,          // vector<vector<u8>> (32-byte LE)     │    │
//! │  │      weights,                // vector<u64>                          │    │
//! │  │      threshold,              // u64                                  │    │
//! │  │      total_weight,           // u64                                  │    │
//! │  │      identity                // vector<u8> (32 bytes)                │    │
//! │  │  ) -> Element<G1>                                                       │    │
//! │  └─────────────────────────────────────────────────────────────────────┘    │
//! │                                    │                                        │
//! │                                    ▼                                        │
//! │  Native Function (this module):                                             │
//! │  1. Parse scalar_shares from vector<vector<u8>> to Vec<Vec<Scalar>>        │
//! │  2. Convert identity Vec<u8> to [u8; 32]                                   │
//! │  3. Call apt_dkg::ibe::reconstruct_ibe_dk() [delegation point]             │
//! │  4. Store result and return handle                                         │
//! │                                    │                                        │
//! │                                    ▼                                        │
//! │  aptos-dkg (Rust SDK):                                                     │
//! │  1. Create DealtSecretKeyShare from each scalar                            │
//! │  2. Call WeightedTranscript::DealtSecretKey::reconstruct()                 │
//! │  3. Derive DK = H(identity)^secret                                         │
//! │                                                                              │
//! └─────────────────────────────────────────────────────────────────────────────┘
//!
//! # Security Considerations
//!
//! - The actual cryptographic operations are performed by `aptos-dkg` to ensure
//!   consistency with the Rust SDK implementation.
//! - This native function only handles data parsing and result storage.
//! - Input validation ensures shares match the expected weights.
//!
//! # Dependencies
//!
//! - `aptos-dkg`: Provides the canonical cryptographic implementation
//! - `blstrs`: BLS12-381 curve arithmetic
//! - `aptos-native-interface`: Move VM integration

use crate::natives::cryptography::algebra::{
    abort_invariant_violated, AlgebraContext, E_TOO_MUCH_MEMORY_USED, MEMORY_LIMIT_IN_BYTES,
};
use crate::store_element;
use aptos_dkg::ibe::reconstruct_ibe_dk;
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, safely_pop_vec_arg, SafeNativeContext, SafeNativeError, SafeNativeResult,
};
use blstrs::Scalar;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;
use std::rc::Rc;

/// Reconstructs an IBE decryption key from threshold scalar shares.
///
/// This is the main native function entry point called from Move code.
/// It parses the Move arguments, delegates to `aptos-dkg` for the cryptographic
/// reconstruction, and stores the result in the algebra context.
///
/// # Arguments
///
/// * `context` - The SafeNativeContext providing gas charging and extensions
/// * `ty_args` - Type arguments (must contain exactly one: G1)
/// * `args` - Arguments pushed from Move:
///   1. `identity` (Vec<u8>) - 32-byte IBE identity
///   2. `total_weight` (u64) - Sum of all validator weights
///   3. `threshold` (u64) - Minimum shares required (unused but kept for API compatibility)
///   4. `weights` (Vec<u64>) - Full weights for ALL validators in network
///   5. `scalar_shares` (Vec<Vec<u8>>) - Scalar shares, each inner vector is 32-byte LE scalars
///   6. `validator_indices` (Vec<u64>) - Indices of participating validators
///
/// # Returns
///
/// Returns `SafeNativeResult<SmallVec<[Value; 1]>>` containing:
/// - `Value::u64(new_handle)` - Handle to the stored G1 element (the reconstructed DK)
///
/// # Errors
///
/// Returns `SafeNativeError::Abort` with:
/// - `E_TOO_MUCH_MEMORY_USED` if storing the result exceeds memory limit
/// - Invariant violation if arguments fail validation
///
/// # Implementation Details
///
/// 1. **Argument Parsing**: Extracts and validates all arguments from the Move VM
/// 2. **Scalar Deserialization**: Converts each 32-byte little-endian vector to a `Scalar`
/// 3. **Delegation**: Calls `aptos_dkg::ibe::reconstruct_ibe_dk()` for crypto operations
/// 4. **Storage**: Stores the resulting G1 point and returns a handle
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
    // ==========================================================================
    // Step 1: Type argument validation
    // ==========================================================================
    // The function expects exactly one type argument: G1
    // This ensures type safety at the Move VM level
    assert_eq!(
        ty_args.len(),
        1,
        "IBE DK reconstruction requires exactly one type argument: G1"
    );

    // ==========================================================================
    // Step 2: Extract arguments from Move VM
    // ==========================================================================
    // Arguments are pushed in a specific order and must be popped in reverse
    let identity: Vec<u8> = safely_pop_arg!(args, Vec<u8>);
    let total_weight: u64 = safely_pop_arg!(args, u64);
    let _threshold: u64 = safely_pop_arg!(args, u64);
    let weights: Vec<u64> = safely_pop_arg!(args, Vec<u64>);
    let scalar_shares: Vec<Vec<u8>> = safely_pop_vec_arg!(args, Vec<u8>);
    let validator_indices: Vec<u64> = safely_pop_arg!(args, Vec<u64>);

    // ==========================================================================
    // Step 3: Validate argument consistency
    // ==========================================================================
    // These checks ensure the input data is well-formed before expensive crypto ops

    // Each participating validator must have corresponding scalar shares
    assert_eq!(
        validator_indices.len(),
        scalar_shares.len(),
        "validator_indices and scalar_shares must have same length: \
         {} validators vs {} share vectors",
        validator_indices.len(),
        scalar_shares.len()
    );

    // At least one validator must participate
    assert!(
        !validator_indices.is_empty(),
        "At least one validator must participate in DK reconstruction"
    );

    // total_weight must match the sum of individual weights
    let computed_total: u64 = weights.iter().copied().sum();
    assert_eq!(
        total_weight, computed_total,
        "total_weight ({}) must match sum of weights ({})",
        total_weight, computed_total
    );

    // ==========================================================================
    // Step 4: Gas charging
    // ==========================================================================
    // Charge for the scalar multiplication operation in DK derivation
    context.charge(ALGEBRA_ARK_BLS12_381_G1_PROJ_SCALAR_MUL)?;

    // ==========================================================================
    // Step 5: Parse scalar shares from Move byte vectors
    // ==========================================================================
    // Each validator's shares are encoded as a flat byte vector with 32-byte
    // little-endian scalar values concatenated together.
    //
    // Example: For validator with shares [s1, s2] (each 32 bytes):
    //   Input:  [0x12, 0x34, ..., 0x56, 0x78, ...] (64 bytes total)
    //   Output: vec![Scalar::from(0x...12), Scalar::from(0x...56)]
    let mut parsed_shares: Vec<Vec<Scalar>> = Vec::with_capacity(scalar_shares.len());

    for shares_bytes in scalar_shares.iter() {
        let num_shares = shares_bytes.len() / 32;
        let mut validator_scalars: Vec<Scalar> = Vec::with_capacity(num_shares);

        let mut offset: usize = 0;
        while offset + 32 <= shares_bytes.len() {
            // Extract 32-byte little-endian scalar
            let mut scalar_bytes = [0u8; 32];
            scalar_bytes.copy_from_slice(&shares_bytes[offset..offset + 32]);

            // Deserialize to Scalar (panics if invalid - caller must provide valid input)
            let scalar = Scalar::from_bytes_le(&scalar_bytes).unwrap();

            validator_scalars.push(scalar);
            offset += 32;
        }

        parsed_shares.push(validator_scalars);
    }

    // ==========================================================================
    // Step 6: Convert identity to fixed-size array
    // ==========================================================================
    // The identity must be exactly 32 bytes (SHA3-256 output size)
    let identity_array: [u8; 32] = identity
        .try_into()
        .map_err(|_| abort_invariant_violated())?;

    // ==========================================================================
    // Step 7: Delegate to apt-dkg for cryptographic reconstruction
    // ==========================================================================
    // This is the critical security-relevant operation:
    // - Virtual player expansion (mapping validators to multiple shares)
    // - Weighted Lagrange interpolation
    // - Secret reconstruction
    // - DK derivation: DK = H(identity)^secret
    //
    // Using apt-dkg ensures the Move VM uses the exact same crypto as the Rust SDK,
    // eliminating any risk of divergence or implementation errors.
    let reconstructed_dk: blstrs::G1Affine = reconstruct_ibe_dk(
        &validator_indices,
        &parsed_shares,
        &weights,
        total_weight,
        &identity_array,
    );

    // ==========================================================================
    // Step 8: Store result and return handle
    // ==========================================================================
    // The G1 point is stored in the algebra context and a handle is returned
    // to the Move VM for later retrieval
    let new_handle: usize = store_element!(context, reconstructed_dk)?;

    Ok(smallvec![Value::u64(new_handle as u64)])
}
