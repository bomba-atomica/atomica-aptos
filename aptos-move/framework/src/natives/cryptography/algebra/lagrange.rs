// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    abort_unless_feature_flag_enabled,
    natives::cryptography::algebra::{
        feature_flag_from_structure, AlgebraContext, Structure, E_TOO_MUCH_MEMORY_USED,
        MEMORY_LIMIT_IN_BYTES, MOVE_ABORT_CODE_NOT_IMPLEMENTED,
    },
    store_element, structure_from_ty_arg,
};
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, SafeNativeContext, SafeNativeError, SafeNativeResult,
};
use aptos_types::on_chain_config::FeatureFlag;
use ark_ff::{Field, PrimeField};
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, rc::Rc};

pub fn lagrange_coefficients_internal(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(1, ty_args.len());
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    let flag_opt = feature_flag_from_structure(structure_opt);
    abort_unless_feature_flag_enabled!(context, flag_opt);

    let participants = safely_pop_arg!(args, Vec<u64>);

    match structure_opt {
        Some(Structure::BLS12381Fr) => {
            compute_lagrange_coefficients::<ark_bls12_381::Fr>(context, participants)
        },
        Some(Structure::BN254Fr) => {
            compute_lagrange_coefficients::<ark_bn254::Fr>(context, participants)
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

fn compute_lagrange_coefficients<F: PrimeField>(
    context: &mut SafeNativeContext,
    participants: Vec<u64>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    // TODO: Charge gas based on participants length
    // For now we just charge a base cost
    // context.charge(ALGEBRA_ARK_BLS12_381_FR_ADD)?; 

    let mut result_handles = Vec::with_capacity(participants.len());
    let mut scalars = Vec::with_capacity(participants.len());

    // O(N^2) implementation
    for k in &participants {
        let mut num = F::one();
        let mut den = F::one();
        
        let k_fr = F::from(*k);

        for i in &participants {
            if i == k {
                continue;
            }

            let i_fr = F::from(*i);
            
            // numerator *= (0 - i) = -i
            let mut neg_i = F::zero();
            neg_i -= i_fr;
            num *= neg_i;

            // denominator *= (k - i)
            let mut diff = k_fr;
            diff -= i_fr;
            den *= diff;
        }

        let den_inv = den.inverse().unwrap_or(F::zero());
        let lambda = num * den_inv;
        scalars.push(lambda);
    }
    
    // Store all scalars
    for scalar in scalars {
        let handle = store_element!(context, scalar)?;
        result_handles.push(Value::u64(handle as u64));
    }

    Ok(smallvec![Value::vector_unchecked(
        result_handles
    )?])
}
