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
        abort_invariant_violated, AlgebraContext, Structure, E_TOO_MUCH_MEMORY_USED,
        MEMORY_LIMIT_IN_BYTES, MOVE_ABORT_CODE_NOT_IMPLEMENTED,
    },
    safe_borrow_element, store_element, structure_from_ty_arg,
};
#[allow(unused_imports)]
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, SafeNativeContext, SafeNativeError, SafeNativeResult,
};
use aptos_types::on_chain_config::FeatureFlag;
use ark_ec::{CurveGroup, PrimeGroup};
use ark_ff::Field;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use num_traits::Zero;
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, rc::Rc};

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

/// Macro to implement DK reconstruction for a specific curve.
macro_rules! reconstruct_dk_internal {
    ($context:expr, $args:ident, $curve:ty, $scalar:ty, $gas:expr) => {{
        let total_weight = safely_pop_arg!($args, u64);
        let threshold = safely_pop_arg!($args, u64);
        let weights: Vec<u64> = safely_pop_arg!($args, Vec<u64>);
        let dk_shares_handles: Vec<u64> = safely_pop_arg!($args, Vec<u64>);
        let validator_indices: Vec<u64> = safely_pop_arg!($args, Vec<u64>);

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

        $context.charge($gas)?;

        // Compute Lagrange coefficients for evaluation at x=0
        // For indices [i_0, i_1, ..., i_{k-1}], the coefficient for i_j is:
        // λ_j = ∏_{m≠j} (0 - i_m) / (i_j - i_m)
        // Since we're evaluating at 0, this simplifies to:
        // λ_j = ∏_{m≠j} (-i_m) / (i_j - i_m)
        let num_shares = validator_indices.len();
        let mut lagrange_coeffs: Vec<$scalar> = Vec::with_capacity(num_shares);

        for j in 0..num_shares {
            let mut numerator = <$scalar>::ONE;
            let mut denominator = <$scalar>::ONE;

            for m in 0..num_shares {
                if m != j {
                    // numerator *= -validator_indices[m]
                    let idx_m_scalar = <$scalar>::from(validator_indices[m] as u64);
                    numerator = numerator * (-idx_m_scalar);

                    // denominator *= (validator_indices[j] - validator_indices[m])
                    let idx_j_scalar = <$scalar>::from(validator_indices[j] as u64);
                    let idx_m_scalar = <$scalar>::from(validator_indices[m] as u64);
                    denominator = denominator * (idx_j_scalar - idx_m_scalar);
                }
            }

            // coefficient = numerator / denominator
            let inv_denominator = denominator.inverse().unwrap();
            let base_coeff = numerator * inv_denominator;

            // Apply weight: λ_j = w_j * base_coeff_j / total_weight
            let weight_scalar = <$scalar>::from(weights[j] as u64);
            let total_weight_scalar = <$scalar>::from(total_weight as u64);
            let weighted_coeff = base_coeff * weight_scalar / total_weight_scalar;

            lagrange_coeffs.push(weighted_coeff);
        }

        // Compute weighted sum: DK = Σ λ_i * dk_share_i
        let mut result = <$curve as Zero>::zero();

        for (i, dk_share_handle) in dk_shares_handles.iter().enumerate() {
            safe_borrow_element!($context, *dk_share_handle as usize, $curve, _ptr, dk_share);

            // Multiply by Lagrange coefficient
            let lambda_bigint: ark_ff::BigInteger256 = lagrange_coeffs[i].into();
            let weighted_share = dk_share.mul_bigint(lambda_bigint);

            // Add to result
            result += weighted_share;
        }

        // Store result and return handle
        let result_affine = result.into_affine();
        let new_handle = store_element!($context, result_affine)?;

        Ok(smallvec![Value::u64(new_handle as u64)])
    }};
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

    let g1_opt = structure_from_ty_arg!(context, &ty_args[0]);
    abort_unless_ibe_enabled!(context, g1_opt);

    match g1_opt {
        Some(Structure::BLS12381G1) => {
            reconstruct_dk_internal!(
                context,
                args,
                ark_bls12_381::G1Projective,
                ark_bls12_381::Fr,
                ALGEBRA_ARK_BLS12_381_G1_PROJ_SCALAR_MUL
            )
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}
