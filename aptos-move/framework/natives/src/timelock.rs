// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives of `aptos_std::ibe`: the timelock (Boneh-Franklin IBE on BLS12-381) checks.
//!
//! Design: `atomica/docs/decisions/aptos-upgrade-ibe-chunky-spec.md` sections 5 and 6.
//! The cryptography lives in `aptos_dkg::ibe`. This file only decodes Move values, charges gas
//! with the framework's existing parameters, and maps errors to aborts or `false`.
//!
//! | Native | Gas charged (existing parameters) |
//! |---|---|
//! | `reconstruct_ibe_dk_internal` | per share: validated G1 decode and one G1 scalar multiplication. Once: the Lagrange work, `d * (log2(d) + 1)` field multiplications for the FFT domain size `d`, priced as scalar multiplications |
//! | `verify_decryption_key_internal` | G1 and G2 validated decode, hash to G1 (base and per byte), one two-term multi-pairing (two pairings) |
//! | `verify_bid_ct_internal` | one G2 validated decode (curve and subgroup check) and one strict Ed25519 verification, with the per byte message hashing |
//!
//! Per-share verification against verification keys is not a Move native. It runs in the Rust
//! validator transaction handler.
//!
//! No FPTX or `aptos-batch-encryption` function is used or exposed here.

use aptos_dkg::ibe::{self, IbeError, MAX_TOTAL_WEIGHT};
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use move_core_types::gas_algebra::{NumArgs, NumBytes};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

/// The type argument of `reconstruct_ibe_dk_internal` is not `bls12381_algebra::G1`.
pub const E_NOT_BLS12381_G1: u64 = 1;
/// Malformed input to `reconstruct_ibe_dk_internal` (shape, bounds, weights, or a share that is
/// not a valid prime-order non-identity G1 point).
pub const E_INVALID_RECONSTRUCTION_INPUT: u64 = 2;

/// Field multiplications priced as one G1 scalar multiplication (92764630 / 18450, rounded up).
const FR_MULS_PER_G1_SCALAR_MUL: u64 = 5030;

fn abort_reconstruction(e: IbeError) -> SafeNativeError {
    SafeNativeError::abort_with_message(E_INVALID_RECONSTRUCTION_INPUT, e.to_string())
}

fn invalid(msg: &str) -> SafeNativeError {
    SafeNativeError::abort_with_message(E_INVALID_RECONSTRUCTION_INPUT, msg.to_string())
}

fn log2_ceil(n: usize) -> u64 {
    (usize::BITS - n.saturating_sub(1).leading_zeros()) as u64
}

/***************************************************************************************************
 * native fun reconstruct_ibe_dk_internal<G1>
 **************************************************************************************************/
fn native_reconstruct_ibe_dk(
    context: &mut SafeNativeContext,
    ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(ty_args.len(), 1);
    debug_assert_eq!(args.len(), 6);

    // Only the BLS12-381 G1 structure is supported.
    let type_tag = context.type_to_type_tag(&ty_args[0])?;
    if type_tag.to_canonical_string() != "0x1::bls12381_algebra::G1" {
        return Err(SafeNativeError::abort(E_NOT_BLS12381_G1));
    }

    let _identity: Vec<u8> = safely_pop_arg!(args, Vec<u8>);
    let total_weight: u64 = safely_pop_arg!(args, u64);
    let threshold: u64 = safely_pop_arg!(args, u64);
    let weights: Vec<u64> = safely_pop_arg!(args, Vec<u64>);
    let shares_value: Vec<Value> = safely_pop_arg!(args, Vec<Value>);
    let validator_indices: Vec<u64> = safely_pop_arg!(args, Vec<u64>);

    // Bound the work before doing it: the number of shares cannot exceed the total weight.
    if shares_value.len() > weights.len() || total_weight > MAX_TOTAL_WEIGHT as u64 {
        return Err(invalid("too many validators or total weight too large"));
    }

    let mut dk_shares: Vec<Vec<Vec<u8>>> = Vec::with_capacity(shares_value.len());
    let mut num_shares: u64 = 0;
    for validator in shares_value {
        let inner: Vec<Value> = validator
            .value_as()
            .map_err(|_| invalid("dk_shares entry is not a vector"))?;
        num_shares += inner.len() as u64;
        if num_shares > MAX_TOTAL_WEIGHT as u64 {
            return Err(invalid("too many shares"));
        }
        let mut v = Vec::with_capacity(inner.len());
        for share in inner {
            v.push(
                share
                    .value_as::<Vec<u8>>()
                    .map_err(|_| invalid("share is not a byte vector"))?,
            );
        }
        dk_shares.push(v);
    }

    // Gas, charged before the decoding and the group arithmetic. Per share: a validated decode
    // and one scalar multiplication. The Lagrange computation costs about `d * (log2 d + 1)`
    // field multiplications for the FFT domain size `d`. A field multiplication costs about
    // 1/5000 of a G1 scalar multiplication, so that work is charged as scalar multiplications.
    for _ in 0..num_shares {
        context.charge(
            ALGEBRA_ARK_BLS12_381_G1_AFFINE_DESER_COMP + ALGEBRA_ARK_BLS12_381_G1_PROJ_SCALAR_MUL,
        )?;
    }
    let domain = (total_weight.max(1) as usize).next_power_of_two() as u64;
    let lagrange_units =
        (domain * (log2_ceil(domain as usize) + 1)).div_ceil(FR_MULS_PER_G1_SCALAR_MUL);
    for _ in 0..lagrange_units {
        context.charge(ALGEBRA_ARK_BLS12_381_G1_PROJ_SCALAR_MUL)?;
    }

    let dk = ibe::reconstruct_ibe_dk_from_g1_shares(
        &validator_indices,
        &dk_shares,
        &weights,
        threshold,
        total_weight,
    )
    .map_err(abort_reconstruction)?;
    Ok(smallvec![Value::vector_u8(ibe::encode_g1(&dk).to_vec())])
}

/***************************************************************************************************
 * native fun verify_decryption_key_internal
 **************************************************************************************************/
fn native_verify_decryption_key(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(_ty_args.is_empty());
    debug_assert_eq!(args.len(), 3);

    let mpk = safely_pop_arg!(args, Vec<u8>);
    let identity = safely_pop_arg!(args, Vec<u8>);
    let dk = safely_pop_arg!(args, Vec<u8>);

    // Validated decodes, hash to G1, and the check e(dk, g2) * e(-H1(id), mpk) = 1.
    context.charge(
        ALGEBRA_ARK_BLS12_381_G1_AFFINE_DESER_COMP
            + ALGEBRA_ARK_BLS12_381_G2_AFFINE_DESER_COMP
            + ALGEBRA_ARK_H2C_BLS12381G1_XMD_SHA256_SSWU_BASE
            + ALGEBRA_ARK_H2C_BLS12381G1_XMD_SHA256_SSWU_PER_MSG_BYTE
                * NumBytes::new(identity.len() as u64)
            + ALGEBRA_ARK_BLS12_381_MULTI_PAIRING_BASE
            + ALGEBRA_ARK_BLS12_381_MULTI_PAIRING_PER_PAIR * NumArgs::from(2),
    )?;

    let ok = identity.len() == ibe::IDENTITY_LEN
        && match (ibe::decode_g1(&dk), ibe::decode_g2(&mpk)) {
            (Ok(dk), Ok(mpk)) => ibe::verify_decryption_key(&dk, &identity, &mpk),
            _ => false,
        };
    Ok(smallvec![Value::bool(ok)])
}

/***************************************************************************************************
 * native fun verify_bid_ct_internal
 **************************************************************************************************/
fn native_verify_bid_ct(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(_ty_args.is_empty());
    debug_assert_eq!(args.len(), 2);

    let ad = safely_pop_arg!(args, Vec<u8>);
    let ct = safely_pop_arg!(args, Vec<u8>);

    // One subgroup-checked G2 decode, and one strict Ed25519 verification over a message made
    // of about |ct| + |ad| bytes.
    let msg_len = ct.len() + ad.len() + 16;
    context.charge(
        ALGEBRA_ARK_BLS12_381_G2_AFFINE_DESER_COMP
            + ED25519_BASE
            + ED25519_PER_PUBKEY_DESERIALIZE * NumArgs::one()
            + ED25519_PER_PUBKEY_SMALL_ORDER_CHECK * NumArgs::one()
            + ED25519_PER_SIG_DESERIALIZE * NumArgs::one()
            + ED25519_PER_SIG_STRICT_VERIFY * NumArgs::one()
            + ED25519_PER_MSG_HASHING_BASE * NumArgs::one()
            + ED25519_PER_MSG_BYTE_HASHING * NumBytes::new(msg_len as u64),
    )?;

    Ok(smallvec![Value::bool(
        ibe::ciphertext::verify_bid_ct(&ct, &ad).is_ok()
    )])
}

pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        (
            "reconstruct_ibe_dk_internal",
            native_reconstruct_ibe_dk as RawSafeNative,
        ),
        (
            "verify_decryption_key_internal",
            native_verify_decryption_key,
        ),
        ("verify_bid_ct_internal", native_verify_bid_ct),
    ];
    builder.make_named_natives(natives)
}
