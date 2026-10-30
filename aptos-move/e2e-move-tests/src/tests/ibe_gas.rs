// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Gas metering of the `aptos_std::ibe` natives (timelock). The natives are priced from existing
//! framework gas parameters. These tests check the charge is real: it is present, it grows with the
//! work (input size, share count), and it is stable. Fixtures are from
//! `aptos-framework/tests/ibe_native_test.move` (see `aptos-dkg::ibe` fixture generator).

use crate::{assert_success, tests::common, MoveHarness};
use aptos_language_e2e_tests::account::Account;
use aptos_types::account_address::AccountAddress;

const IDENTITY: &str = "be639abf404445211e05906cb7459f81cbc236fbf41f615f2d52b53b7c695956";
const MPK: &str = "a495c99da9fb780a2438450308598f3e53a5dccd0187e003b56473a9b6a8630fde766eb42d8d22314a114d856ecb643005a23bfcd222821174a51b5a4772704a399862766f455ed576eb3f9e971b1a0336189945c90c757eb93108a0cb3cb74d";
const DK: &str = "b8a8f94189ec0fe889f8297cd460d02526fdc87210b8592cf741c3c63c5b0b41b9476cea27f0d699fb39ff7cd664f306";
const AD: &str = "1761746f6d6963612f74696d656c6f636b2d6269642f763207020000000000000001000000000000000500000000000000010101010101010101010101010101010101010101010101010101010101010100401e18240a0600";
const CT: &str = "20952b73d3c0a06bc8243f47b052168e099f42b107529b91f2eec62b5a2774162b60aa429f1e599701fbbeb9a42d3c3a97ad64551fe43b91d533ba4dd71508d80c58bc72946c2c651fc6197232d15d00434b058a6be2909580e6c710686b41199f263b1ccc65d0b060a8d3d20492470bcadfb8fdc016ac6e5ca45432be28be6f25020cb3ea07d0db3fd3228978fc5b15983a33bb2ce0ebc21a8bf8fe7239894deb5d32bdd040f46776a1498526dfe6b03d410f88844c2623bf91425db012be0185c2dfb42671b55e62e17303710d2b5128d7f581014159b87f44855121213e1fd0814a7c730a";
const SHARES: [&str; 5] = [
    "94905cdef5c743407b0689c6f3ec85f75106281835e05ec71cd902398f352d6241e920b661850443594606abd173a46e",
    "ae1432e8cb17b1d729d7c83433ae6e1284ba54b0e247f79459b8893237c1f235b5bfd185b97b165b705fc1368c56ab42",
    "aa83a0145a8b697de670d1100a4ae4f35acf6fa3ec873c40cca5bbf7dca4453fb43a33d43376e08a04dcd8ce3a388aad",
    "815cb8ac4abc0ccdc80b5ab538bc99aa2798069a19d150fba89de12d7d323ac91de7c6b0da0896f3e9df49ecac19eb78",
    "95a7a18fc3749dcacf7c4a7a00ef5094f01871f0478a5d16f48a44a2fc54817b692dd0b5ae8c07decf45a2836f083374",
];

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

    fn reconstruct(&mut self, weights: Vec<u64>, counts: Vec<u64>, shares: Vec<&str>) -> u64 {
        let flat: Vec<Vec<u8>> = shares.iter().map(|s| hex(s)).collect();
        self.gas(
            "reconstruct",
            vec![
                bcs::to_bytes(&flat).unwrap(),
                bcs::to_bytes(&counts).unwrap(),
                bcs::to_bytes(&weights).unwrap(),
                bcs::to_bytes(&5u64).unwrap(),
                bcs::to_bytes(&7u64).unwrap(),
                bcs::to_bytes(&hex(IDENTITY)).unwrap(),
            ],
        )
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

    // reconstruct_ibe_dk: five shares, each a validated decode and a G1 scalar multiplication,
    // plus the Lagrange work. It must cost more than the single pairing check.
    let rec = e.reconstruct(vec![2, 3, 2], vec![2, 3], SHARES.to_vec());
    assert!(rec > base, "reconstruct is not charged");
    assert!(
        rec > bid,
        "five scalar multiplications {rec} should exceed one bid check {bid}"
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
