// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Native functions for IBE (Identity-Based Encryption) operations.
//!
//! This module provides native functions for:
//! - Decryption key reconstruction from threshold shares
//! - IBE decryption operations
//!
//! These functions wrap the cryptographic operations from aptps-dkg to enable
//! on-chain IBE operations without writing complex crypto in Move.

use crate::{
    abort_unless_feature_flag_enabled,
    natives::cryptography::algebra::{
        abort_invariant_violated, AlgebraContext, Structure, MOVE_ABORT_CODE_NOT_IMPLEMENTED,
    },
    safe_borrow_element, store_element, structure_from_ty_arg,
};
#[allow(unused_imports)]
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, safely_pop_vec, SafeNativeContext, SafeNativeError, SafeNativeResult,
};
use aptos_types::on_chain_config::FeatureFlag;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

fn feature_flag_of_ibe(g1_opt: Option<Structure>) -> Option<FeatureFlag> {
    match g1_opt {
        Some(Structure::BLS12381G1) => Some(FeatureFlag::BLS12_381_STRUCTURES),
        _ => None,
    }
}

macro_rules! abort_unless_ibe_enabled {
    ($context:ident, $g1_opt:expr) => {
        let flag_opt = feature_flag_of_ibe($g1_opt);
        abort_unless_feature_flag_enabled!($context, flag_opt);
    };
}

/// Reconstruct an IBE decryption key from threshold shares using Lagrange interpolation.
///
/// This function implements the Lagrange interpolation logic from aptos-dkg:
/// DK = Σ λ_i * dk_share_i
///
/// where λ_i are Lagrange coefficients computed based on validator indices and weights.
///
/// # Arguments
/// * `validator_indices` - Vector of validator indices (u64)
/// * `dk_shares` - Vector of G1 element handles (decryption key shares)
/// * `weights` - Vector of validator weights (u64)
/// * `threshold` - The threshold number of shares required
/// * `total_weight` - Total weight of all validators
///
/// # Returns
/// * Handle to the reconstructed G1 decryption key element
#[allow(clippy::result_large_err)]
pub fn reconstruct_ibe_dk_internal(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(ty_args.len(), 1, "Expected exactly one type argument: G1");

    let total_weight = safely_pop_arg!(args, u64);
    let threshold = safely_pop_arg!(args, u64);
    let weights: Vec<u64> = safely_pop_vec!(args);
    let dk_shares_handles: Vec<u64> = safely_pop_vec!(args);
    let validator_indices: Vec<u64> = safely_pop_arg!(args, u64);

    // Validate inputs
    assert_eq!(
        validator_indices.len(),
        dk_shares_handles.len(),
        "validator_indices and dk_shares must have same length"
    );
    assert_eq!(
        validator_indices.len(),
        weights.len(),
        "validator_indices and weights must have same length"
    );
    assert!(
        validator_indices.len() >= threshold as usize,
        "Must have at least threshold shares"
    );

    let g1_opt = structure_from_ty_arg!(context, &ty_args[0]);
    abort_unless_ibe_enabled!(context, g1_opt);

    match g1_opt {
        Some(Structure::BLS12381G1) => {
            context.charge(ALGEBRA_ARK_BLS12_381_G1_PROJ_SCALAR_MUL)?;
            reconstruct_impl::<ark_bls12_381::G1Projective>(
                context,
                &validator_indices,
                &dk_shares_handles,
                &weights,
                threshold,
                total_weight,
            )
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

fn reconstruct_impl<G: ark_ec::Group>(
    context: &mut SafeNativeContext,
    validator_indices: &[u64],
    dk_shares_handles: &[u64],
    weights: &[u64],
    threshold: u64,
    total_weight: u64,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    use crate::algebra::lagrange::lagrange_coefficients;

    // Compute Lagrange coefficients
    let batch_size = validator_indices.len();
    let indices_usize: Vec<usize> = validator_indices.iter().map(|&x| x as usize).collect();
    let base_coeffs = lagrange_coefficients(batch_size, &indices_usize, &blstrs::Scalar::ZERO);

    // Apply weights: λ_i = w_i * base_coeff_i / total_weight
    let mut lagrange_coeffs = Vec::with_capacity(base_coeffs.len());
    for (i, base_coeff) in base_coeffs.iter().enumerate() {
        let weighted =
            *base_coeff * blstrs::Scalar::from(weights[i]) / blstrs::Scalar::from(total_weight);
        lagrange_coeffs.push(weighted);
    }

    // Compute weighted sum: DK = Σ λ_i * dk_share_i
    let mut result = G::identity();

    for (i, dk_share_handle) in dk_shares_handles.iter().enumerate() {
        safe_borrow_element!(context, *dk_share_handle as usize, G, _, dk_share);

        // Multiply by Lagrange coefficient
        let lambda_bigint: ark_ff::BigInteger256 = lagrange_coeffs[i].into();
        let weighted_share = dk_share.mul_bigint(lambda_bigint);

        // Add to result
        result += weighted_share;
    }

    // Store result and return handle
    let result_affine = result.to_affine();
    let new_handle = store_element!(context, result_affine)?;

    Ok(smallvec![Value::u64(new_handle as u64)])
}
