// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Native function for IBE (Identity-Based Encryption) decryption key reconstruction.
//!
//! This module provides the Move VM native function for reconstructing IBE decryption keys
//! from validator-submitted DK shares (G1 points).

use crate::natives::cryptography::algebra::abort_invariant_violated;
use aptos_dkg::ibe::reconstruct_ibe_dk_from_g1_shares;
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, SafeNativeContext, SafeNativeError, SafeNativeResult,
};
use blstrs::G1Affine;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

/// Reconstructs an IBE decryption key from G1 DK shares.
///
/// This is the main native function entry point called from Move code.
/// It validates and deserializes G1 DK shares, delegates to `aptos-dkg`
/// for the weighted Lagrange reconstruction, and returns the result.
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

    let _identity: Vec<u8> = safely_pop_arg!(args, Vec<u8>);
    let total_weight: u64 = safely_pop_arg!(args, u64);
    let threshold: u64 = safely_pop_arg!(args, u64);
    let weights: Vec<u64> = safely_pop_arg!(args, Vec<u64>);
    // Nested DK shares: vector<vector<vector<u8>>> - pop manually
    let dk_shares_raw: Vec<Value> = args
        .pop_back()
        .ok_or_else(|| {
            SafeNativeError::InvariantViolation(
                abort_invariant_violated().with_message("Missing dk_shares argument".to_string()),
            )
        })?
        .value_as()
        .map_err(|_| {
            SafeNativeError::InvariantViolation(
                abort_invariant_violated()
                    .with_message("Failed to cast dk_shares to Vec<Value>".to_string()),
            )
        })?;
    let validator_indices: Vec<u64> = safely_pop_arg!(args, Vec<u64>);

    // Convert raw Values to Vec<Vec<Vec<u8>>>
    // dk_shares_raw[i] is a Value representing vector<vector<u8>>
    let mut dk_shares_nested: Vec<Vec<Vec<u8>>> = Vec::new();
    for validator_value in dk_shares_raw {
        let validator_shares_raw: Vec<Value> = validator_value.value_as().map_err(|_| {
            SafeNativeError::InvariantViolation(
                abort_invariant_violated()
                    .with_message("Failed to convert validator shares to Vec<Value>".to_string()),
            )
        })?;

        let mut validator_shares: Vec<Vec<u8>> = Vec::new();
        for share_value in validator_shares_raw {
            let share_bytes: Vec<u8> = share_value.value_as().map_err(|_| {
                SafeNativeError::InvariantViolation(
                    abort_invariant_violated()
                        .with_message("Failed to convert share to Vec<u8>".to_string()),
                )
            })?;
            validator_shares.push(share_bytes);
        }
        dk_shares_nested.push(validator_shares);
    }

    // Flatten nested shares into virtual player IDs and G1 points
    let mut virtual_player_ids: Vec<u64> = Vec::new();
    let mut dk_shares: Vec<Vec<u8>> = Vec::new();

    // Use a temporary WeightedConfig to get player starting indices
    let weights_usize: Vec<usize> = weights.iter().map(|w| *w as usize).collect();
    let wconfig =
        aptos_dkg::pvss::WeightedConfig::new(threshold as usize, weights_usize).map_err(|e| {
            SafeNativeError::InvariantViolation(
                abort_invariant_violated().with_message(format!("Invalid weighted config: {}", e)),
            )
        })?;

    use aptos_dkg::pvss::traits::SecretSharingConfig;

    // Validate that validator_indices and dk_shares have matching lengths
    if validator_indices.len() != dk_shares_nested.len() {
        return Err(SafeNativeError::InvariantViolation(
            abort_invariant_violated().with_message(format!(
                "validator_indices length ({}) doesn't match dk_shares length ({})",
                validator_indices.len(),
                dk_shares_nested.len()
            )),
        ));
    }

    for (i, &validator_idx) in validator_indices.iter().enumerate() {
        let player = wconfig.get_player(validator_idx as usize);
        let weight = wconfig.get_player_weight(&player);
        let starting_index = wconfig.get_player_starting_index(&player);

        let validator_shares = &dk_shares_nested[i];
        if validator_shares.len() != weight {
            return Err(SafeNativeError::InvariantViolation(
                abort_invariant_violated().with_message(format!(
                    "Validator {} has {} shares but weight is {}",
                    validator_idx,
                    validator_shares.len(),
                    weight
                )),
            ));
        }

        for (j, share_bytes) in validator_shares.iter().enumerate() {
            if share_bytes.len() != 48 {
                return Err(SafeNativeError::InvariantViolation(
                    abort_invariant_violated().with_message(format!(
                        "DK share must be 48 bytes (compressed G1), got {}",
                        share_bytes.len()
                    )),
                ));
            }

            // Charge gas for each virtual player contribution (scalar multiplication in Lagrange sum)
            context.charge(ALGEBRA_ARK_BLS12_381_G1_PROJ_SCALAR_MUL)?;

            virtual_player_ids.push((starting_index + j) as u64);
            dk_shares.push(share_bytes.clone());
        }
    }

    // Delegate to apt-dkg for G1-based reconstruction
    let reconstructed_dk: G1Affine =
        match reconstruct_ibe_dk_from_g1_shares(&virtual_player_ids, &dk_shares, total_weight) {
            Ok(dk) => dk,
            Err(e) => {
                return Err(SafeNativeError::InvariantViolation(
                    abort_invariant_violated()
                        .with_message(format!("IBE DK reconstruction failed: {}", e)),
                ));
            },
        };

    // Serialize result as 48-byte compressed G1
    let dk_bytes = reconstructed_dk.to_compressed().to_vec();

    Ok(smallvec![Value::vector_u8(dk_bytes)])
}
