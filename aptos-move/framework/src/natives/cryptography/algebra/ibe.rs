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
    let flag_opt = feature_flag_of_ibe(g1_opt);
    abort_unless_feature_flag_enabled!(context, flag_opt);

    match g1_opt {
        Some(Structure::BLS12381G1) => {
            let total_weight = safely_pop_arg!(args, u64);
            let threshold = safely_pop_arg!(args, u64);
            let weights: Vec<u64> = safely_pop_arg!(args, Vec<u64>);
            let dk_shares_handles: Vec<u64> = safely_pop_arg!(args, Vec<u64>);
            let validator_indices: Vec<u64> = safely_pop_arg!(args, Vec<u64>);

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

            context.charge(ALGEBRA_ARK_BLS12_381_G1_PROJ_SCALAR_MUL)?;

            let num_shares = validator_indices.len();
            let mut lagrange_coeffs: Vec<ark_bls12_381::Fr> = Vec::with_capacity(num_shares);

            for j in 0..num_shares {
                let mut numerator: ark_bls12_381::Fr = ark_bls12_381::Fr::ONE;
                let mut denominator: ark_bls12_381::Fr = ark_bls12_381::Fr::ONE;

                for m in 0..num_shares {
                    if m != j {
                        let idx_m_scalar = ark_bls12_381::Fr::from(validator_indices[m]);
                        numerator *= -idx_m_scalar;

                        let idx_j_scalar = ark_bls12_381::Fr::from(validator_indices[j]);
                        let idx_m_scalar = ark_bls12_381::Fr::from(validator_indices[m]);
                        denominator *= idx_j_scalar - idx_m_scalar;
                    }
                }

                let inv_denominator = denominator.inverse().unwrap();
                let base_coeff = numerator * inv_denominator;

                let weight_scalar = ark_bls12_381::Fr::from(weights[j]);
                let total_weight_scalar = ark_bls12_381::Fr::from(total_weight);
                let weighted_coeff = base_coeff * weight_scalar / total_weight_scalar;

                lagrange_coeffs.push(weighted_coeff);
            }

            let mut result = ark_bls12_381::G1Projective::zero();

            for (i, dk_share_handle) in dk_shares_handles.iter().enumerate() {
                safe_borrow_element!(
                    context,
                    *dk_share_handle as usize,
                    ark_bls12_381::G1Projective,
                    _ptr,
                    dk_share
                );

                let scalar_bigint: ark_ff::BigInteger256 = lagrange_coeffs[i].into();
                let weighted_share = dk_share.mul_bigint(scalar_bigint);

                result += weighted_share;
            }

            let new_handle = store_element!(context, result)?;

            Ok(smallvec![Value::u64(new_handle as u64)])
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}
