// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Client side tool for the V2 timelock bid wrapper. It is what `atomica/scripts/timelock-smoke-test.sh`
//! calls. Every value is hex on the command line and on stdout.
//!
//! ```text
//! timelock_bid identity <chain_id> <epoch> <timelock_id> <deadline_us>
//! timelock_bid ad <chain_id> <epoch> <timelock_id> <auction_id> <bidder_hex32> <deadline_us>
//! timelock_bid encrypt <mpk_hex> <identity_hex> <ad_hex> <plaintext_hex>
//! timelock_bid verify <ciphertext_hex> <ad_hex>
//! timelock_bid check-key <dk_hex> <identity_hex> <mpk_hex>
//! timelock_bid decrypt <dk_hex> <identity_hex> <ad_hex> <ciphertext_hex>
//! ```
//!
//! `verify`, `check-key` and `decrypt` exit with status 1 and print nothing on stdout when the check fails.

use aptos_dkg::ibe::{
    ciphertext::{decrypt_bid, encrypt_bid, BidCiphertext},
    compute_identity, decode_g1, decode_g2, verify_decryption_key, IDENTITY_LEN,
};
use std::process::exit;

fn unhex(s: &str) -> Vec<u8> {
    hex::decode(s.trim_start_matches("0x")).unwrap_or_else(|e| fail(&format!("bad hex: {e}")))
}

fn fail(msg: &str) -> ! {
    eprintln!("{msg}");
    exit(1)
}

fn num<T: std::str::FromStr>(s: &str) -> T {
    s.parse()
        .unwrap_or_else(|_| fail(&format!("bad number {s}")))
}

fn identity(s: &str) -> [u8; IDENTITY_LEN] {
    unhex(s)
        .try_into()
        .unwrap_or_else(|_| fail("identity must be 32 bytes"))
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let arg = |i: usize| {
        a.get(i)
            .map(String::as_str)
            .unwrap_or_else(|| fail("missing argument"))
    };
    match arg(0) {
        "identity" => {
            let id = compute_identity(num(arg(1)), num(arg(2)), num(arg(3)), num(arg(4)));
            println!("{}", hex::encode(id));
        },
        "ad" => {
            let bidder: [u8; 32] = unhex(arg(5))
                .try_into()
                .unwrap_or_else(|_| fail("bidder must be 32 bytes"));
            let ad = bcs::to_bytes(&(
                "atomica/timelock-bid/v2",
                num::<u8>(arg(1)),
                num::<u64>(arg(2)),
                num::<u64>(arg(3)),
                num::<u64>(arg(4)),
                bidder,
                num::<u64>(arg(6)),
            ))
            .unwrap();
            println!("{}", hex::encode(ad));
        },
        "encrypt" => {
            let mpk = decode_g2(&unhex(arg(1))).unwrap_or_else(|e| fail(&format!("mpk: {e:?}")));
            let ct = encrypt_bid(
                &mpk,
                &identity(arg(2)),
                &unhex(arg(3)),
                &unhex(arg(4)),
                &mut rand::rngs::OsRng,
            )
            .unwrap_or_else(|e| fail(&format!("encrypt: {e:?}")));
            println!("{}", hex::encode(ct.to_bytes()));
        },
        "verify" => {
            let ct = BidCiphertext::from_bytes(&unhex(arg(1)))
                .unwrap_or_else(|e| fail(&format!("parse: {e:?}")));
            ct.verify(&unhex(arg(2)))
                .unwrap_or_else(|e| fail(&format!("verify: {e:?}")));
            println!("ok");
        },
        "check-key" => {
            let dk = decode_g1(&unhex(arg(1))).unwrap_or_else(|e| fail(&format!("dk: {e:?}")));
            let mpk = decode_g2(&unhex(arg(3))).unwrap_or_else(|e| fail(&format!("mpk: {e:?}")));
            if !verify_decryption_key(&dk, &identity(arg(2)), &mpk) {
                fail("key does not verify");
            }
            println!("ok");
        },
        "decrypt" => {
            let dk = decode_g1(&unhex(arg(1))).unwrap_or_else(|e| fail(&format!("dk: {e:?}")));
            let ct = BidCiphertext::from_bytes(&unhex(arg(4)))
                .unwrap_or_else(|e| fail(&format!("parse: {e:?}")));
            let pt = decrypt_bid(&dk, &identity(arg(2)), &unhex(arg(3)), &ct)
                .unwrap_or_else(|e| fail(&format!("decrypt: {e:?}")));
            println!("{}", hex::encode(pt));
        },
        other => fail(&format!("unknown command {other}")),
    }
}
