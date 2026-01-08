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

/// Serialize Fp element to 48 bytes (big-endian).
fn serialize_fp_big_endian(fp: &ark_bls12_381::Fq) -> [u8; 48] {
    fp.into_bigint().to_bytes_be()
}

/// Serialize an Fp2 element to 96 bytes.
/// Fp2 = (c0: Fp, c1: Fp) -> c0 || c1 (each 48 bytes, big-endian)
fn serialize_fp2_big_endian(fp2: &ark_bls12_381::Fq2) -> [u8; 96] {
    let mut result = [0u8; 96];
    let c0 = serialize_fp_big_endian(fp2.coeffs().0);
    let c1 = serialize_fp_big_endian(fp2.coeffs().1);
    result[0..48].copy_from_slice(&c0);
    result[48..96].copy_from_slice(&c1);
    result
}

/// Serialize Fp12 to 576 bytes in canonical format.
///
/// This matches the Rust implementation in `crates/aptos-dkg/src/ibe/fp12_raw_serialization.rs`
/// and the TypeScript implementation in `atomica/timelock-tests/src/ibe-crypto.ts`.
///
/// Fp12 structure (via tower extension):
/// - Fp12 = (c0: Fp6, c1: Fp6)
/// - Fp6 = (c0: Fp2, c1: Fp2, c2: Fp2)
/// - Fp2 = (c0: Fp, c1: Fp)
///
/// Output format (576 bytes):
/// fp12.c0.c0 || fp12.c0.c1 || fp12.c0.c2 || fp12.c1.c0 || fp12.c1.c1 || fp12.c1.c2
/// (each Fp2 is 96 bytes = 2 * 48 byte Fp elements in big-endian)
fn serialize_fp12_canonical(fp12: &ark_bls12_381::Fq12) -> Vec<u8> {
    use ark_bls12_381::fq12::Fp12Parameters;
    let mut result = Vec::with_capacity(576);

    // Access the internal representation via coefficients()
    // Fp12 is represented as c0 + c1 * w where w^4 - gamma = 0
    let coeffs = fp12.coeffs();

    // Serialize c0 (Fp6 = 3 Fp2 = 288 bytes)
    // Fp6 is stored as [a, b, c] where value = a + b * u + c * u^2
    let c0_fps: &[ark_bls12_381::Fq6] = &coeffs[0..3];
    for fp6 in c0_fps {
        let fp6_coeffs = fp6.coeffs();
        // Each Fp6 contains 3 Fp2 elements
        result.extend_from_slice(&serialize_fp2_big_endian(&fp6_coeffs[0]));
        result.extend_from_slice(&serialize_fp2_big_endian(&fp6_coeffs[1]));
        result.extend_from_slice(&serialize_fp2_big_endian(&fp6_coeffs[2]));
    }

    // Serialize c1 (Fp6 = 3 Fp2 = 288 bytes)
    let c1_fps: &[ark_bls12_381::Fq6] = &coeffs[3..6];
    for fp6 in c1_fps {
        let fp6_coeffs = fp6.coeffs();
        // Each Fp6 contains 3 Fp2 elements
        result.extend_from_slice(&serialize_fp2_big_endian(&fp6_coeffs[0]));
        result.extend_from_slice(&serialize_fp2_big_endian(&fp6_coeffs[1]));
        result.extend_from_slice(&serialize_fp2_big_endian(&fp6_coeffs[2]));
    }

    debug_assert_eq!(result.len(), 576);
    result
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

        // Serialize K using canonical Fp12 serialization
        // This matches the Rust implementation's format for cross-language compatibility
        $context.charge($serialize_gas_cost * 12)?; // 12 Fp elements
        let k_bytes = serialize_fp12_canonical(&k_gt);

        // Keccak256 Hash
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

#[allow(clippy::result_large_err)]
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
