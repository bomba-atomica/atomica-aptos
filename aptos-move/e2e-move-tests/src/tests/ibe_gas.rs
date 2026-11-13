// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Gas metering of the `aptos_std::ibe` natives (timelock). The natives are priced from existing
//! framework gas parameters. These tests check the charge is real: it is present, it grows with the
//! work (input size), and it is stable. Fixtures are from
//! `aptos-framework/tests/ibe_native_test.move` (see `aptos-dkg::ibe` fixture generator).

use crate::{assert_success, assert_vm_status, tests::common, MoveHarness};
use aptos_language_e2e_tests::account::Account;
use aptos_types::{
    account_address::AccountAddress, transaction::TransactionStatus, vm_status::StatusCode,
};

const MPK: &str = "a495c99da9fb780a2438450308598f3e53a5dccd0187e003b56473a9b6a8630fde766eb42d8d22314a114d856ecb643005a23bfcd222821174a51b5a4772704a399862766f455ed576eb3f9e971b1a0336189945c90c757eb93108a0cb3cb74d";
const DK: &str = "b8a8f94189ec0fe889f8297cd460d02526fdc87210b8592cf741c3c63c5b0b41b9476cea27f0d699fb39ff7cd664f306";
const AD: &str = "1761746f6d6963612f74696d656c6f636b2d6269642f763207020000000000000001000000000000000500000000000000010101010101010101010101010101010101010101010101010101010101010100401e18240a0600";
const CT: &str = "20952b73d3c0a06bc8243f47b052168e099f42b107529b91f2eec62b5a2774162b60aa429f1e599701fbbeb9a42d3c3a97ad64551fe43b91d533ba4dd71508d80c58bc72946c2c651fc6197232d15d00434b058a6be2909580e6c710686b41199f263b1ccc65d0b060a8d3d20492470bcadfb8fdc016ac6e5ca45432be28be6f25020cb3ea07d0db3fd3228978fc5b15983a33bb2ce0ebc21a8bf8fe7239894deb5d32bdd040f46776a1498526dfe6b03d410f88844c2623bf91425db012be0185c2dfb42671b55e62e17303710d2b5128d7f581014159b87f44855121213e1fd0814a7c730a";
/// `ibe_config::compute_identity(4, 2, 1, 1_000_000)`, the identity `CT` is encrypted to.
const IDENTITY: &str = "be639abf404445211e05906cb7459f81cbc236fbf41f615f2d52b53b7c695956";

/// The largest number of `ibe::decrypt_bid` calls that fit in one transaction under
/// `max_execution_gas`. The doc comment of `ibe::decrypt_bid` repeats it as `largest batch: N`.
const LARGEST_DECRYPT_BATCH: u64 = 15;
const IBE_MOVE: &str =
    include_str!("../../../framework/aptos-stdlib/sources/cryptography/ibe.move");
/// Growth of the associated data in the slope test. A transaction holds at most 64 KiB.
const AD_GROWTH: usize = 32_768;

fn hex(s: &str) -> Vec<u8> {
    hex::decode(s).unwrap()
}

struct Env {
    h: MoveHarness,
    acc: Account,
}

impl Env {
    fn new() -> Self {
        let mut h = MoveHarness::new();
        let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());
        assert_success!(h.publish_package(&acc, &common::test_dir_path("ibe_gas.data/p")));
        Self { h, acc }
    }

    fn gas(&mut self, f: &str, args: Vec<Vec<u8>>) -> u64 {
        let txn = self.h.create_entry_function(
            &self.acc,
            str::parse(&format!("0xbeef::ibe_gas::{f}")).unwrap(),
            vec![],
            args,
        );
        self.h.evaluate_gas_signed(txn)
    }

    fn decrypt_many_args(n: u64) -> Vec<Vec<u8>> {
        let ads = vec![hex(AD); n as usize];
        let cts = vec![hex(CT); n as usize];
        vec![
            v(hex(DK)),
            v(hex(IDENTITY)),
            bcs::to_bytes(&ads).unwrap(),
            bcs::to_bytes(&cts).unwrap(),
        ]
    }

    fn run_decrypt_many(&mut self, n: u64) -> TransactionStatus {
        let txn = self.h.create_entry_function(
            &self.acc,
            str::parse("0xbeef::ibe_gas::decrypt_many").unwrap(),
            vec![],
            Self::decrypt_many_args(n),
        );
        self.h.run(txn)
    }
}

fn v(b: Vec<u8>) -> Vec<u8> {
    bcs::to_bytes(&b).unwrap()
}

#[test]
fn ibe_natives_are_metered_and_scale_with_work() {
    let mut e = Env::new();
    let base = e.gas("noop", vec![]);

    // verify_bid_ct: the Ed25519 message hashing is charged per byte of ad.
    let bid = e.gas("bid_ct", vec![v(hex(CT)), v(hex(AD))]);
    assert!(bid > base, "verify_bid_ct is not charged: {bid} vs {base}");

    // verify_decryption_key: two pairings, a hash to G1 and two validated decodes. A wrong-length
    // identity returns false but is still charged per byte.
    let dk_args = |id: Vec<u8>| vec![v(hex(DK)), v(id), v(hex(MPK))];
    let dk_short = e.gas("dk_check", dk_args(vec![7u8; 32]));
    let dk_long = e.gas("dk_check", dk_args(vec![7u8; 4096]));
    assert!(
        dk_short > base + 1,
        "verify_decryption_key is not charged: {dk_short} vs {base}"
    );
    assert!(
        dk_long > dk_short,
        "no per-byte identity charge: {dk_long} vs {dk_short}"
    );
    // Pairing work dominates a bid check: it must cost more than the Ed25519 path.
    assert!(
        dk_short > bid,
        "pairing check {dk_short} should exceed bid check {bid}"
    );
}

/// A decrypt costs more than the submission check, and its extra charge per byte of `ad` is at least
/// `hash.sha2_256.per_byte` (the HKDF and AES-128-GCM price) over that of `verify_bid_ct`.
#[test]
fn decrypt_bid_costs_more_than_verify_bid_ct_and_grows_with_the_ciphertext_body() {
    let mut e = Env::new();
    let (_, params) = e.h.get_gas_params();
    let per_byte = u64::from(params.natives.move_stdlib.hash_sha2_256_per_byte);
    let scaling = u64::from(params.vm.txn.gas_unit_scaling_factor);

    let short_ad = hex(AD);
    let mut long_ad = short_ad.clone();
    long_ad.extend(vec![0u8; AD_GROWTH]);
    let decrypt_args = |ad: &[u8]| vec![v(hex(DK)), v(hex(IDENTITY)), v(ad.to_vec()), v(hex(CT))];
    let verify_args = |ad: &[u8]| vec![v(hex(CT)), v(ad.to_vec())];

    let dec_short = e.gas("decrypt", decrypt_args(&short_ad));
    let dec_long = e.gas("decrypt", decrypt_args(&long_ad));
    let ver_short = e.gas("bid_ct_any", verify_args(&short_ad));
    let ver_long = e.gas("bid_ct_any", verify_args(&long_ad));
    println!("decrypt {dec_short} -> {dec_long}, verify_bid_ct {ver_short} -> {ver_long}");

    // One pairing and a G1 decode on top of the submission check.
    assert!(
        dec_short > ver_short,
        "decrypt {dec_short} should exceed verify_bid_ct {ver_short}"
    );
    assert!(dec_long > dec_short, "no per-byte decrypt charge");
    let dec_growth = dec_long - dec_short;
    let ver_growth = ver_long
        .checked_sub(ver_short)
        .expect("verify_bid_ct shrank");
    let extra = dec_growth
        .checked_sub(ver_growth)
        .expect("decrypt grows less than verify_bid_ct");
    let floor = (AD_GROWTH as u64 * per_byte / scaling).saturating_sub(1);
    assert!(
        extra >= floor,
        "decrypt grows by {extra} gas over verify_bid_ct for {AD_GROWTH} bytes, below {floor}"
    );
}

/// The doc lines (`///`) right above `public fun decrypt_bid(` in `ibe.move`.
fn decrypt_bid_doc() -> String {
    let lines: Vec<&str> = IBE_MOVE.lines().collect();
    let at = lines
        .iter()
        .position(|l| l.trim_start().starts_with("public fun decrypt_bid("))
        .expect("ibe.move declares decrypt_bid");
    let mut doc: Vec<&str> = lines[..at]
        .iter()
        .rev()
        .take_while(|l| l.trim_start().starts_with("///"))
        .copied()
        .collect();
    doc.reverse();
    doc.join("\n")
}

/// B decrypts fit in one transaction under `max_execution_gas` and B + 1 do not. B bounds the batch
/// size of the product `open_bids`.
#[test]
fn largest_decrypt_batch_fits_max_execution_gas() {
    let mut e = Env::new();
    let (_, params) = e.h.get_gas_params();
    let scaling = u64::from(params.vm.txn.gas_unit_scaling_factor);
    let max_execution_gas = u64::from(params.vm.txn.max_execution_gas);
    assert_eq!(max_execution_gas, 9_200_000_000);

    let one = e.gas("decrypt_many", Env::decrypt_many_args(1));
    let two = e.gas("decrypt_many", Env::decrypt_many_args(2));
    let per_decrypt = (two - one) * scaling;
    let b = max_execution_gas / per_decrypt;
    println!(
        "decrypt_many(1) = {one}, decrypt_many(2) = {two}, per decrypt {per_decrypt} internal gas, largest batch {b}"
    );

    assert_success!(e.run_decrypt_many(b));
    assert_vm_status!(
        e.run_decrypt_many(b + 1),
        StatusCode::EXECUTION_LIMIT_REACHED
    );
    assert_eq!(
        b, LARGEST_DECRYPT_BATCH,
        "update LARGEST_DECRYPT_BATCH and the `largest batch:` line of the ibe::decrypt_bid doc"
    );
    let doc = decrypt_bid_doc();
    assert!(
        doc.contains(&format!("largest batch: {LARGEST_DECRYPT_BATCH}.")),
        "the doc of ibe::decrypt_bid must say `largest batch: {LARGEST_DECRYPT_BATCH}.`:\n{doc}"
    );
}

#[test]
fn ibe_native_gas_is_deterministic() {
    let mut e = Env::new();
    let args = || vec![v(hex(CT)), v(hex(AD))];
    let a = e.gas("bid_ct", args());
    let b = e.gas("bid_ct", args());
    assert_eq!(a, b);
}
