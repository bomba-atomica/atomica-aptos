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
//! | `verify_decryption_key_internal` | G1 and G2 validated decode, hash to G1 (base and per byte), one two-term multi-pairing (two pairings) |
//! | `verify_bid_ct_internal` | one G2 validated decode (curve and subgroup check) and one strict Ed25519 verification, with the per byte message hashing |
//!
//! Per-share verification against verification keys is not a Move native. It runs in the Rust
//! validator transaction handler.
//!
//! No FPTX or `aptos-batch-encryption` function is used or exposed here.

// SCOUT(#227) DOC: the gas table in the module doc above gains a `decrypt_bid_internal` row.
use aptos_dkg::ibe;
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeResult,
};
use move_core_types::gas_algebra::{NumArgs, NumBytes};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

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

// SCOUT(#227) ADD: `fn native_decrypt_bid(context, ty_args, args)` for `ibe::decrypt_bid_internal(dk, identity, ad, ct)`. It pops four byte vectors, charges gas before any decoding, calls `aptos_dkg::ibe::decrypt_bid_ct` and returns the plaintext or a failure flag. It never aborts on bad input. No Rust stub here: this crate is consensus code, and the stub would be dead until the Move declaration lands in the same commit.
// SCOUT(#227) EDIT: `Option` is an enum in this move-stdlib (`public enum Option`), so a native cannot cheaply return one. Return `(bool, vector<u8>)` and let `ibe::decrypt_bid` build the `Option` in Move.
// SCOUT(#227) EDIT: gas comes from existing parameters only: validated G1 and G2 decode, strict Ed25519 with per-byte hashing, one pairing, and a per-byte charge over `ct` and `ad`. No existing parameter prices HKDF or AES-GCM, so the per-byte parameter must be chosen and named.
// SCOUT(#228) EDIT: the product `auction::open_bids` pays this charge once per bid; the 100-bid gas test bounds the batch size it may pass.
// SCOUT(#227) ADD: register `("decrypt_bid_internal", native_decrypt_bid)` in the table below, in the same commit as the Move native declaration.
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        (
            "verify_decryption_key_internal",
            native_verify_decryption_key as RawSafeNative,
        ),
        ("verify_bid_ct_internal", native_verify_bid_ct),
    ];
    builder.make_named_natives(natives)
}
