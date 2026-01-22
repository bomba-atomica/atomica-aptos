// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Native functions for IBE (Identity-Based Encryption) operations.
//!
//! This module provides native functions for:
//! - Decryption key reconstruction from threshold shares
//!
//! The DK reconstruction is delegated to apt-dkg's canonical implementation.
//! This native function only handles serialization/deserialization between
//! the Move VM's storage and apt-dkg's format.

use crate::{
    abort_unless_feature_flag_enabled,
    natives::cryptography::algebra::{
        abort_invariant_violated, AlgebraContext, Structure, E_TOO_MUCH_MEMORY_USED,
        MEMORY_LIMIT_IN_BYTES, MOVE_ABORT_CODE_NOT_IMPLEMENTED,
    },
    safe_borrow_element, store_element, structure_from_ty_arg,
};
use aptos_dkg::ibe::reconstruct_ibe_dk;
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, SafeNativeContext, SafeNativeError, SafeNativeResult,
};
use aptos_types::on_chain_config::FeatureFlag;
use ark_ec::CurveGroup;
use ark_serialize::CanonicalSerialize;
use blstrs::G1Affine;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, rc::Rc};

fn feature_flag_of_ibe(g1_opt: Option<Structure>) -> Option<FeatureFlag> {
    match g1_opt {
        Some(Structure::BLS12381G1) => Some(FeatureFlag::BLS12_381_STRUCTURES),
        _ => None,
    }
}

/// Reconstruct an IBE decryption key from threshold shares.
///
/// This function delegates to apt-dkg's canonical implementation.
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
            let _threshold = safely_pop_arg!(args, u64);
            let weights: Vec<u64> = safely_pop_arg!(args, Vec<u64>);
            let dk_shares_handles: Vec<u64> = safely_pop_arg!(args, Vec<u64>);
            let validator_indices: Vec<u64> = safely_pop_arg!(args, Vec<u64>);

            assert_eq!(
                validator_indices.len(),
                dk_shares_handles.len(),
                "validator_indices and dk_shares must have same length"
            );
            assert!(validator_indices.len() >= 1, "Must have at least one validator");

            context.charge(ALGEBRA_ARK_BLS12_381_G1_PROJ_SCALAR_MUL)?;

            // Convert input handles to blstrs format
            let mut dk_shares: Vec<G1Affine> = Vec::with_capacity(dk_shares_handles.len());
            for &handle in dk_shares_handles.iter() {
                safe_borrow_element!(
                    context,
                    handle as usize,
                    ark_bls12_381::G1Projective,
                    _ptr,
                    dk_share
                );
                let affine = dk_share.into_affine();
                let mut bytes = [0u8; 48];
                affine
                    .serialize_compressed(&mut bytes[..])
                    .map_err(|_| abort_invariant_violated())?;
                let ct_option = G1Affine::from_compressed(&bytes);
                if ct_option.is_some().unwrap_u8() == 0 {
                    return Err(abort_invariant_violated().into());
                }
                dk_shares.push(ct_option.unwrap());
            }

            // Call apt-dkg's canonical implementation
            let reconstructed_dk =
                reconstruct_ibe_dk(&validator_indices, &dk_shares, &weights, total_weight);

            // Store result directly (no conversion needed)
            let new_handle = store_element!(context, reconstructed_dk)?;

            Ok(smallvec![Value::u64(new_handle as u64)])
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}
                    shares_for_validator.push(ct_option.unwrap());
                }
                dk_shares.push(shares_for_validator);
            }

            // Call apt-dkg's canonical implementation
            let reconstructed_dk =
                reconstruct_ibe_dk(&validator_indices, &dk_shares, &weights, total_weight);

            // Store result directly (no conversion needed)
            let new_handle = store_element!(context, reconstructed_dk)?;

            Ok(smallvec![Value::u64(new_handle as u64)])
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}
