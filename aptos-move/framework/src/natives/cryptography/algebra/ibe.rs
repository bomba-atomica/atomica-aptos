// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    abort_unless_feature_flag_enabled,
    natives::cryptography::algebra::{
        abort_invariant_violated, AlgebraContext, Structure, MOVE_ABORT_CODE_NOT_IMPLEMENTED,
    },
    safe_borrow_element, structure_from_ty_arg,
};
#[allow(unused_imports)]
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, SafeNativeContext, SafeNativeError, SafeNativeResult,
};
use aptos_types::on_chain_config::FeatureFlag;
use ark_ec::{pairing::Pairing, CurveGroup};
use ark_serialize::CanonicalSerialize;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;
use tiny_keccak::{Hasher, Keccak};

fn feature_flag_of_ibe(
    g1_opt: Option<Structure>,
    g2_opt: Option<Structure>,
    gt_opt: Option<Structure>,
) -> Option<FeatureFlag> {
    match (g1_opt, g2_opt, gt_opt) {
        (Some(Structure::BLS12381G1), Some(Structure::BLS12381G2), Some(Structure::BLS12381Gt)) => {
            Some(FeatureFlag::BLS12_381_STRUCTURES)
        },
        _ => None,
    }
}

macro_rules! abort_unless_ibe_enabled {
    ($context:ident, $g1_opt:expr, $g2_opt:expr, $gt_opt:expr) => {
        let flag_opt = feature_flag_of_ibe($g1_opt, $g2_opt, $gt_opt);
        abort_unless_feature_flag_enabled!($context, flag_opt);
    };
}

macro_rules! decrypt_internal_impl {
    (
        $context:expr,
        $args:ident,
        $pairing:ty,
        $g1_projective:ty,
        $g2_projective:ty,
        $pairing_gas_cost:expr,
        $g1_proj_to_affine_gas_cost:expr,
        $g2_proj_to_affine_gas_cost:expr,
        $serialize_gas_cost:expr
    ) => {{
        let ciphertext = safely_pop_arg!($args, Vec<u8>);
        let sig_element_handle = safely_pop_arg!($args, u64) as usize;
        let u_element_handle = safely_pop_arg!($args, u64) as usize;

        // Load U (G1)
        safe_borrow_element!(
            $context,
            u_element_handle,
            $g1_projective,
            u_element_ptr,
            u_element
        );
        $context.charge($g1_proj_to_affine_gas_cost)?;
        let u_element_affine = u_element.into_affine();

        // Load Signature (G2)
        safe_borrow_element!(
            $context,
            sig_element_handle,
            $g2_projective,
            sig_element_ptr,
            sig_element
        );
        $context.charge($g2_proj_to_affine_gas_cost)?;
        let sig_element_affine = sig_element.into_affine();

        // Pairing: K = e(U, Sig)
        $context.charge($pairing_gas_cost)?;
        let k_gt = <$pairing>::pairing(u_element_affine, sig_element_affine).0;

        // Serialize K
        $context.charge($serialize_gas_cost)?;
        let mut k_bytes = Vec::new();
        k_gt.serialize_uncompressed(&mut k_bytes)
            .map_err(|_e| abort_invariant_violated())?;

        // Keccak256 Hash
        // Charge some gas for hashing? Reusing serialization cost as proxy for now + per-byte?
        // Ideally we define specific gas. For PoC, we will assume it is covered.
        let mut sha3 = Keccak::v256();
        sha3.update(&k_bytes);
        let mut mask = [0u8; 32];
        sha3.finalize(&mut mask);

        // XOR
        let mut result = Vec::with_capacity(ciphertext.len());
        for (i, byte) in ciphertext.iter().enumerate() {
            result.push(byte ^ mask[i % 32]);
        }

        Ok(smallvec![Value::vector_u8(result)])
    }};
}

pub fn decrypt_internal(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(3, ty_args.len());
    let g1_opt = structure_from_ty_arg!(context, &ty_args[0]);
    let g2_opt = structure_from_ty_arg!(context, &ty_args[1]);
    let gt_opt = structure_from_ty_arg!(context, &ty_args[2]);
    abort_unless_ibe_enabled!(context, g1_opt, g2_opt, gt_opt);

    match (g1_opt, g2_opt, gt_opt) {
        (Some(Structure::BLS12381G1), Some(Structure::BLS12381G2), Some(Structure::BLS12381Gt)) => {
            decrypt_internal_impl!(
                context,
                args,
                ark_bls12_381::Bls12_381,
                ark_bls12_381::G1Projective,
                ark_bls12_381::G2Projective,
                ALGEBRA_ARK_BLS12_381_PAIRING,
                ALGEBRA_ARK_BLS12_381_G1_PROJ_TO_AFFINE,
                ALGEBRA_ARK_BLS12_381_G2_PROJ_TO_AFFINE,
                ALGEBRA_ARK_BLS12_381_FQ12_SERIALIZE
            )
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}
